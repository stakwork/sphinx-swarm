use crate::{
    cmd::{CreateEc2InstanceInfo, CreateEc2InstanceRes},
    ec2::{add_new_tags_to_instance, Ec2Tags},
    service::swarm_reserver::utils::check_reserve_swarm_flag_set,
    state::Super,
};
use anyhow::{anyhow, Error};

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

    // if password passed update child swarm password
    // if env passed update child swarm env
    // if vanity address passed update HOST in .env
    // if vanity address passed update route53 record with vanity address and delete old record
    // stop all child services
    // restart main swarm service to pick up new .env vars
    // move reserved swarm from reserved to normal swarm list
    // return swarm details
    Err(anyhow!("Not implemented yet"))
}
