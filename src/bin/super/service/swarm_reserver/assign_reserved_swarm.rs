use std::collections::HashMap;

use crate::{
    cmd::{CreateEc2InstanceInfo, CreateEc2InstanceRes},
    ec2::{add_new_tags_to_instance, Ec2Tags},
    route53::{add_domain_name_to_route53, delete_multiple_route53_records},
    service::{
        swarm_reserver::{
            call_child_swarm::call_child_swarm_to_activate_new_swarm,
            utils::check_reserve_swarm_flag_set,
        },
        update_child_swarm::handle_update_child_swarm,
    },
    state::{RemoteStack, Super},
};
use anyhow::{anyhow, Error};
use sphinx_swarm::cmd::AssignSwarmNewDetails;

/// Revert the EC2 instance Name tag back to the original reserved name so that
/// a failed assignment does not poison `instance_with_swarm_name_exists`.
async fn revert_instance_name_tag(instance_id: &str, original_name: &str) {
    let revert_tags = vec![Ec2Tags {
        key: "Name".to_string(),
        value: original_name.to_string(),
    }];
    if let Err(e) = add_new_tags_to_instance(instance_id, revert_tags).await {
        log::error!("Failed to revert Name tag after assign failure: {}", e);
    }
}

pub async fn handle_assign_reserved_swarm(
    info: &CreateEc2InstanceInfo,
    state: &mut Super,
) -> Result<CreateEc2InstanceRes, Error> {
    // ── Phase 1: Validate everything — zero side effects ──────────────────────

    if !check_reserve_swarm_flag_set() {
        return Err(anyhow!(
            "Reserve Swarm Flag not set, we can't assign a reserved swarm at the momemnt"
        ));
    }

    if state.reserved_instances.is_none() {
        return Err(anyhow!("No reserved instances available at the moment"));
    }

    if state
        .reserved_instances
        .clone()
        .unwrap()
        .available_instances
        .len()
        <= 0
    {
        return Err(anyhow!("No reserved instances available at the moment"));
    }

    // Validate graph_mindset env vars before any AWS mutation
    let cln_btc_url: Option<String> = if info.workspace_type.as_deref() == Some("graph_mindset") {
        let url = sphinx_swarm::utils::getenv("CLN_MAINNET_BTC")
            .map_err(|_| anyhow!("CLN_MAINNET_BTC env var required for graph_mindset workspace"))?;
        Some(url)
    } else {
        None
    };

    // ── Phase 2: Build data structures — zero side effects ────────────────────

    let selected_reserved_instance = state
        .reserved_instances
        .clone()
        .unwrap()
        .available_instances[0]
        .clone();

    // Capture the original name now so we can revert the tag on failure
    let original_name = selected_reserved_instance.swarm_id();

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

    if info.vanity_address.is_some() {
        if let Some(envs_map) = envs.as_mut() {
            envs_map.insert("HOST".to_string(), info.vanity_address.clone().unwrap());
        }
    }

    // inject graph_mindset env vars using the already-validated cln_btc_url
    if let Some(btc_url) = cln_btc_url {
        let envs_map = envs.get_or_insert_with(HashMap::new);
        envs_map.insert("GRAPH_MINDSET_ONLY".to_string(), "true".to_string());
        envs_map.insert("SECOND_BRAIN_ONLY".to_string(), "false".to_string());
        // generate unique seed for this swarm's CLN node
        let seed = sphinx_swarm::secrets::hex_secret_32();
        envs_map.insert("SEED".to_string(), seed);
        envs_map.insert("CLN_MAINNET_BTC".to_string(), btc_url);
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

    // ── Phase 3: Execute side effects in order ────────────────────────────────

    // update tag name on AWS
    add_new_tags_to_instance(&selected_reserved_instance.instance_id, tags).await?;

    // if password passed update child swarm password
    // if env passed update child swarm env
    // if vanity address passed update HOST in .env
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
            // Revert Name tag so retry is not blocked by stale tag
            revert_instance_name_tag(
                &selected_reserved_instance.instance_id,
                &original_name,
            )
            .await;
            // TODO: send error message via a bot to a tribe
            return Err(anyhow!(
                "Failed to call child swarm to activate new swarm: {}",
                err
            ));
        }
    };

    if !set_value_res.success {
        // TODO: send error message via a bot to a tribe
        log::error!(
            "Failed to set new password/env on child swarm: {}",
            set_value_res.message
        );
    }
    let mut host = selected_reserved_instance.host.clone();
    let mut default_host = selected_reserved_instance.default_host.clone();

    // restart main swarm service to pick up new .env vars
    let _ = match handle_update_child_swarm(&swarm_details).await {
        Ok(_) => {}
        Err(e) => {
            log::error!(
                "Failed to update child swarm after assigning reserved swarm: {}",
                e.to_string()
            );
        }
    };

    if info.vanity_address.is_some() && selected_reserved_instance.ip_address.is_some() {
        // update route53
        let vanity_address = info.vanity_address.clone().unwrap();
        let _ = match add_domain_name_to_route53(
            vec![vanity_address.clone()],
            &selected_reserved_instance.ip_address.clone().unwrap(),
        )
        .await
        {
            Ok(_) => {}
            Err(err) => {
                // Revert Name tag so retry is not blocked by stale tag
                revert_instance_name_tag(
                    &selected_reserved_instance.instance_id,
                    &original_name,
                )
                .await;
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

    // move reserved swarm from reserved to normal swarm list
    let swarm_id = format!("swarm{}", selected_reserved_instance.swarm_number);
    state.stacks.push(RemoteStack {
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
    });
    let x_api_key = selected_reserved_instance.x_api_key.clone();
    let ec2_id = selected_reserved_instance.instance_id.clone();

    state
        .reserved_instances
        .as_mut()
        .unwrap()
        .available_instances
        .remove(0);

    Ok(CreateEc2InstanceRes {
        swarm_id: swarm_id,
        x_api_key,
        address: host,
        ec2_id,
    })
}
