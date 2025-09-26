use std::collections::HashMap;

use crate::{aws_util::make_aws_client, state::InstanceFromAws};
use anyhow::{anyhow, Error};
use aws_sdk_ec2::types::{Filter, Instance, Tag};
use chrono;
use serde::{Deserialize, Serialize};
use sphinx_swarm::utils::getenv;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default, Clone)]
pub struct Ec2Tags {
    pub key: String,
    pub value: String,
}

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

pub async fn add_new_tags_to_instance(
    instance_id: &str,
    passed_tags: Vec<Ec2Tags>,
) -> Result<(), Error> {
    let client = make_aws_client().await?;

    let mut tags: Vec<Tag> = Vec::new();

    for tag in passed_tags {
        tags.push(Tag::builder().key(tag.key).value(tag.value).build());
    }

    // Apply the tag
    client
        .create_tags()
        .resources(instance_id)
        .set_tags(Some(tags)) // here we pass the Vec<Tag>
        .send()
        .await?;

    log::info!("Tags added to instance {}", instance_id);
    Ok(())
}

pub async fn get_instances_from_aws_by_swarm_tag_and_return_hash_map(
) -> Result<HashMap<String, InstanceFromAws>, Error> {
    let key = getenv("SWARM_TAG_KEY")?;
    let value = getenv("SWARM_TAG_VALUE")?;
    let aws_instances = get_swarms_by_tag(&key, &value).await?;

    let mut aws_instances_hashmap: HashMap<String, InstanceFromAws> = HashMap::new();

    for aws_instance in aws_instances {
        aws_instances_hashmap.insert(aws_instance.instance_id.clone(), aws_instance.clone());
    }

    Ok(aws_instances_hashmap)
}

pub async fn stop_ec2_instance_and_tag(instance_id: &str) -> Result<(), Error> {
    let client = make_aws_client().await?;

    // Stop the instance
    client
        .stop_instances()
        .instance_ids(instance_id)
        .send()
        .await?;

    // Add the DeletedOn tag with current date
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tags = vec![Ec2Tags {
        key: "DeletedOn".to_string(),
        value: current_date,
    }];

    add_new_tags_to_instance(instance_id, tags).await?;

    log::info!("Instance {} stopped and tagged with DeletedOn", instance_id);
    Ok(())
}
