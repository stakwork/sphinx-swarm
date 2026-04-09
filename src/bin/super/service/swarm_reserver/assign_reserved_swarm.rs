use std::collections::HashMap;

use crate::{
    cmd::{CreateEc2InstanceInfo, CreateEc2InstanceRes},
    ec2::{add_new_tags_to_instance, Ec2Tags},
    route53::{add_domain_name_to_route53, delete_multiple_route53_records},
    service::swarm_reserver::{
        call_child_swarm::call_child_swarm_to_activate_new_swarm,
        utils::check_reserve_swarm_flag_set,
    },
    state::{state_write, AvailableInstances, RemoteStack},
};
use anyhow::{anyhow, Error};
use sphinx_swarm::cmd::AssignSwarmNewDetails;

/// Restore a claimed reserved instance back to the pool on failure.
async fn rollback_reserved_instance(proj: &str, instance: AvailableInstances) {
    state_write(proj, |state| {
        if let Some(ri) = &mut state.reserved_instances {
            ri.available_instances.push(instance);
        }
    })
    .await;
}

pub async fn handle_assign_reserved_swarm(
    info: &CreateEc2InstanceInfo,
    proj: &str,
) -> Result<CreateEc2InstanceRes, Error> {
    if !check_reserve_swarm_flag_set() {
        return Err(anyhow!(
            "Reserve Swarm Flag not set, we can't assign a reserved swarm at the momemnt"
        ));
    }

    // Claim the first available reserved instance atomically (remove from pool)
    let selected_reserved_instance = state_write(proj, |state| {
        state
            .reserved_instances
            .as_mut()
            .and_then(|ri| {
                if ri.available_instances.is_empty() {
                    None
                } else {
                    Some(ri.available_instances.remove(0))
                }
            })
    })
    .await;

    let selected_reserved_instance = match selected_reserved_instance {
        Some(i) => i,
        None => return Err(anyhow!("No reserved instances available at the moment")),
    };

    let mut tags = vec![
        Ec2Tags {
            key: "Name".to_string(),
            value: info
                .name
                .clone()
                .unwrap_or_else(|| selected_reserved_instance.swarm_id()),
        },
        Ec2Tags {
            key: "log_group".to_string(),
            value: format!("/swarms/{}", selected_reserved_instance.swarm_number),
        },
    ];

    if info.testing == Some(true) {
        tags.push(Ec2Tags {
            key: "testing".to_string(),
            value: "true".to_string(),
        });
    }

    // update tag name on AWS (I/O — no lock)
    if let Err(e) = add_new_tags_to_instance(&selected_reserved_instance.instance_id, tags).await {
        rollback_reserved_instance(proj, selected_reserved_instance).await;
        return Err(e);
    }

    let mut envs: Option<HashMap<String, String>> = info.env.clone();
    if envs.is_none() {
        envs = Some(HashMap::new());
    }
    let mut new_password: Option<String> = None;
    let mut old_password: Option<String> = None;

    if info.password.is_some() {
        new_password = info.password.clone();
        old_password = Some(selected_reserved_instance.admin_password.clone());
    }

    if let Some(vanity) = &info.vanity_address {
        let envs_map = envs.get_or_insert_with(HashMap::new);
        envs_map.insert("HOST".to_string(), vanity.clone());
        envs_map.insert("NAV_BOLTWALL_SHARED_HOST".to_string(), vanity.clone());
    }

    // inject owner pubkey if present
    if let Some(pubkey) = &info.owner_pubkey {
        let envs_map = envs.get_or_insert_with(HashMap::new);
        envs_map.insert("OWNER_PUBKEY".to_string(), pubkey.clone());
    }

    if envs.is_some() && envs.clone().unwrap().is_empty() {
        envs = None;
    }

    let swarm_details = RemoteStack {
        host: "".to_string(),
        note: None,
        ec2: None,
        user: selected_reserved_instance.user.clone(),
        pass: selected_reserved_instance.pass.clone(),
        default_host: selected_reserved_instance.default_host.clone(),
        ec2_instance_id: selected_reserved_instance.instance_id.clone(),
        public_ip_address: selected_reserved_instance.ip_address.clone(),
        private_ip_address: None,
        id: Some(format!("swarm{}", selected_reserved_instance.swarm_number)),
        deleted: None,
        route53_domain_names: None,
        owner_pubkey: info.owner_pubkey.clone(),
        workspace_type: info.workspace_type.clone(),
        cln_pubkey: None,
    };

    // Call child swarm to activate (I/O — no lock)
    let set_value_res = match call_child_swarm_to_activate_new_swarm(
        &swarm_details,
        AssignSwarmNewDetails {
            new_password,
            old_password,
            env: envs,
        },
    )
    .await
    {
        Ok(res) => res,
        Err(err) => {
            rollback_reserved_instance(proj, selected_reserved_instance).await;
            return Err(anyhow!(
                "Failed to call child swarm to activate new swarm: {}",
                err
            ));
        }
    };

    if !set_value_res.success {
        log::error!(
            "Failed to set new password/env on child swarm: {}",
            set_value_res.message
        );
    }

    let mut host = selected_reserved_instance.host.clone();
    let mut default_host = selected_reserved_instance.default_host.clone();

    if info.vanity_address.is_some() && selected_reserved_instance.ip_address.is_some() {
        // update route53 (I/O — no lock)
        let vanity_address = info.vanity_address.clone().unwrap();
        match add_domain_name_to_route53(
            vec![vanity_address.clone()],
            &selected_reserved_instance.ip_address.clone().unwrap(),
        )
        .await
        {
            Ok(_) => {}
            Err(err) => {
                rollback_reserved_instance(proj, selected_reserved_instance).await;
                return Err(anyhow!(
                    "Failed to add domain name to route53: {}",
                    err.to_string()
                ));
            }
        };
        host = vanity_address.clone();
        default_host = format!("{}:8800", vanity_address);
        // and delete old record
        match delete_multiple_route53_records(vec![selected_reserved_instance.host.clone()]).await {
            Ok(_) => {}
            Err(err) => {
                log::error!(
                    "Failed to delete old route53 record for {}: {}",
                    selected_reserved_instance.host.clone(),
                    err.to_string()
                );
            }
        };
    }

    // Write: add to active stacks (brief write lock)
    // Already removed from reserved pool in the claim step above.
    let swarm_id = format!("swarm{}", selected_reserved_instance.swarm_number);
    let new_stack = RemoteStack {
        host: host.clone(),
        note: None,
        ec2: Some(selected_reserved_instance.instance_type.clone()),
        user: selected_reserved_instance.user.clone(),
        pass: selected_reserved_instance.pass.clone(),
        default_host,
        ec2_instance_id: selected_reserved_instance.instance_id.clone(),
        public_ip_address: selected_reserved_instance.ip_address.clone(),
        private_ip_address: None,
        id: Some(swarm_id.clone()),
        deleted: Some(false),
        route53_domain_names: Some(vec![host.clone()]),
        owner_pubkey: info.owner_pubkey.clone(),
        workspace_type: info.workspace_type.clone(),
        cln_pubkey: None,
    };
    let x_api_key = selected_reserved_instance.x_api_key.clone();
    let ec2_id = selected_reserved_instance.instance_id.clone();

    state_write(proj, |state| {
        state.stacks.push(new_stack);
    })
    .await;

    Ok(CreateEc2InstanceRes {
        swarm_id: swarm_id,
        x_api_key,
        address: host,
        ec2_id,
    })
}
