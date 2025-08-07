use crate::{aws_util::make_aws_client, state::InstanceFromAws};
use anyhow::{anyhow, Error};
use aws_sdk_ec2::types::{Filter, Instance};
use sphinx_swarm::utils::getenv;

pub async fn get_swarms_by_tag(key: &str, value: &str) -> Result<Vec<InstanceFromAws>, Error> {
    let mut instances: Vec<InstanceFromAws> = vec![];

    let instances_by_tag = match get_ec2_instance_by_tag(key, value).await {
        Ok(response) => response,
        Err(err) => return Err(err),
    };

    for instance in instances_by_tag {
        let instance_id = instance.instance_id.unwrap_or_default();
        let mut instance_type = "".to_string();
        let public_ip_address = instance.public_ip_address.unwrap_or_default();
        let private_ip_address = instance.private_ip_address.unwrap_or_default();

        if instance.instance_type.is_some() {
            instance_type = instance.instance_type.unwrap().to_string()
        }

        instances.push(InstanceFromAws {
            instance_id,
            instance_type,
            public_ip_address,
            private_ip_address,
        });
    }

    return Ok(instances);
}

pub async fn instance_with_swarm_name_exists(swarm_name: &str) -> Result<bool, Error> {
    let key = getenv("SWARM_TAG_KEY")?;
    let value = getenv("SWARM_TAG_VALUE")?;
    let ec2_instances = match get_ec2_instance_by_tag(&key, &value).await {
        Ok(instances) => instances,
        Err(err) if err.to_string() == "No instances found with the given tag." => {
            return Ok(false)
        }
        Err(err) => return Err(err),
    };

    if ec2_instances.is_empty() {
        return Ok(false);
    }

    let normalized_swarm_name = swarm_name.to_lowercase();
    let alt_swarm_name = if normalized_swarm_name.ends_with("-swarm") {
        normalized_swarm_name.trim_end_matches("-swarm").to_string()
    } else {
        format!("{}-swarm", normalized_swarm_name)
    };

    for instance in ec2_instances {
        if let Some(tags) = instance.tags {
            for tag in tags {
                if tag.key.is_some() && tag.value.is_some() {
                    let value = tag.value.unwrap().to_lowercase();
                    if tag.key.unwrap() == "Name".to_string()
                        && (value == normalized_swarm_name || value == alt_swarm_name)
                    {
                        return Ok(true);
                    }
                }
            }
        }
    }
    return Ok(false);
}

pub async fn get_ec2_instance_by_tag(key: &str, value: &str) -> Result<Vec<Instance>, Error> {
    let mut instances: Vec<Instance> = Vec::new();
    let client = make_aws_client().await?;

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
                instances.push(instance);
            }
        } else {
            log::error!("Instances do not exist")
        }
    }

    return Ok(instances);
}
