use anyhow::Result;

use crate::ec2::{add_new_tags_to_instance, Ec2Tags};
use crate::state;

/// Migrate all existing swarms to have a `log_group` EC2 tag.
///
/// Derives the log group from `RemoteStack.id` by stripping the "swarm" prefix:
///   id = "swarmAb3xYz" => SWARM_NUMBER = "Ab3xYz" => log_group = "/swarms/Ab3xYz"
///
/// Also tags reserved instances that haven't been assigned yet.
pub async fn migrate_log_group_tags() {
    log::info!("Starting log_group tag migration for existing swarms...");

    let state = state::STATE.lock().await;

    // Collect (instance_id, swarm_number) pairs for active swarms
    let mut to_tag: Vec<(String, String)> = Vec::new();

    for stack in &state.stacks {
        if stack.deleted == Some(true) {
            continue;
        }
        if stack.ec2_instance_id.is_empty() {
            continue;
        }
        if let Some(id) = &stack.id {
            if let Some(swarm_number) = id.strip_prefix("swarm") {
                to_tag.push((stack.ec2_instance_id.clone(), swarm_number.to_string()));
            } else {
                log::warn!(
                    "Swarm id '{}' does not start with 'swarm', skipping log_group tag",
                    id
                );
            }
        }
    }

    // Also tag reserved instances
    if let Some(reserved) = &state.reserved_instances {
        for instance in &reserved.available_instances {
            if !instance.instance_id.is_empty() && !instance.swarm_number.is_empty() {
                to_tag.push((
                    instance.instance_id.clone(),
                    instance.swarm_number.clone(),
                ));
            }
        }
    }

    drop(state);

    let total = to_tag.len();
    log::info!("Found {} instances to tag with log_group", total);

    let mut success_count = 0;
    let mut error_count = 0;

    for (instance_id, swarm_number) in to_tag {
        let log_group = format!("/swarms/{}", swarm_number);
        match tag_instance_with_log_group(&instance_id, &log_group).await {
            Ok(()) => {
                success_count += 1;
                log::info!(
                    "Tagged instance {} with log_group={}",
                    instance_id,
                    log_group
                );
            }
            Err(e) => {
                error_count += 1;
                log::error!(
                    "Failed to tag instance {} with log_group={}: {}",
                    instance_id,
                    log_group,
                    e
                );
            }
        }
    }

    log::info!(
        "log_group tag migration complete: {} succeeded, {} failed out of {} total",
        success_count,
        error_count,
        total
    );
}

async fn tag_instance_with_log_group(instance_id: &str, log_group: &str) -> Result<()> {
    let tags = vec![Ec2Tags {
        key: "log_group".to_string(),
        value: log_group.to_string(),
    }];
    add_new_tags_to_instance(instance_id, tags).await
}
