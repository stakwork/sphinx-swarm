use crate::{aws_util::make_aws_client, state::InstanceFromAws};
use anyhow::{anyhow, Error};
use aws_sdk_ec2::types::Filter;

pub async fn get_swarms_by_tag(key: &str, value: &str) -> Result<Vec<InstanceFromAws>, Error> {
    let client = make_aws_client().await?;

    let mut instances: Vec<InstanceFromAws> = vec![];

    let tag_filter = Filter::builder()
        .name(format!("tag:{}", key))
        .values(format!("{}", value))
        .build();

    let response = client
        .describe_instances()
        .filters(tag_filter)
        .send()
        .await?;

    if response.reservations().is_empty() {
        log::error!("No instances found with the given tag.");
        return Err(anyhow!("No instances found with the given tag."));
    }

    for reservation in response.reservations.unwrap() {
        if !reservation.instances().is_empty() {
            for instance in reservation.instances.unwrap() {
                if instance.public_ip_address.is_some()
                    && instance.instance_id.is_some()
                    && instance.instance_type.is_some()
                {
                    instances.push(InstanceFromAws {
                        instacne_id: instance.instance_id.unwrap(),
                        intance_type: instance.instance_type.unwrap().to_string(),
                    });
                }
            }
        } else {
            log::error!("Instances do not exist")
        }
    }

    return Ok(instances);
}
