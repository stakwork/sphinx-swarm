use std::collections::HashMap;

use crate::{
    cmd::{CreateEc2InstanceInfo, CreateEc2InstanceRes},
    ec2::{add_new_tags_to_instance, Ec2Tags},
    service::swarm_reserver::{
        call_child_swarm::call_child_swarm_to_activate_new_swarm,
        utils::check_reserve_swarm_flag_set,
    },
    state::{RemoteStack, Super},
};
use anyhow::{anyhow, Error};
use sphinx_swarm::cmd::AssignSwarmNewDetails;

pub async fn handle_assign_reserved_swarm(
    info: &CreateEc2InstanceInfo,
    state: &mut Super,
) -> Result<CreateEc2InstanceRes, Error> {
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

    let selected_reserved_instance = state
        .reserved_instances
        .clone()
        .unwrap()
        .available_instances[0]
        .clone();

    // update tag name on AWS
    add_new_tags_to_instance(
        &selected_reserved_instance.instance_id,
        vec![Ec2Tags {
            key: "Name".to_string(),
            value: info.name.clone(),
        }],
    )
    .await?;
    let mut envs: Option<HashMap<String, String>> = info.env.clone();
    if envs.is_none() {
        envs = Some(HashMap::new());
    }
    let mut new_password: Option<String> = None;
    let mut old_password: Option<String> = None;

    if info.password.is_some() {
        new_password = info.password.clone();
        old_password = Some(selected_reserved_instance.admin_password.clone());
        if let Some(envs_map) = envs.as_mut() {
            envs_map.insert("PASSWORD".to_string(), info.password.clone().unwrap());
        }
    }

    if info.vanity_address.is_some() {
        if let Some(envs_map) = envs.as_mut() {
            envs_map.insert("HOST".to_string(), info.vanity_address.clone().unwrap());
        }
    }

    // if password passed update child swarm password
    // if env passed update child swarm env
    // if vanity address passed update HOST in .env
    // if vanity address passed update route53 record with vanity address and delete old record
    // stop all child services
    let set_value_res = match call_child_swarm_to_activate_new_swarm(
        &RemoteStack {
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
        },
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
            // TODO: send error message via a bot to a tribe
            return Err(anyhow!(
                "Failed to call child swarm to activate new swarm: {}",
                err
            ));
        }
    };

    if !set_value_res.success {
        // TODO: send error message via a bot to a tribe
    }
    // restart main swarm service to pick up new .env vars
    // update route53 and delete old record
    // move reserved swarm from reserved to normal swarm list
    // return swarm details
    Err(anyhow!("Not implemented yet"))
}
