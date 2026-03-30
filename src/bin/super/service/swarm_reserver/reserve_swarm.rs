use anyhow::Result;
use tokio::time::{sleep, Duration};

use crate::{
    ec2::get_instances_from_aws_by_swarm_tag_and_return_hash_map,
    put_config_file,
    route53::{add_domain_name_to_route53, delete_multiple_route53_records},
    service::swarm_reserver::utils::generate_random_secret,
    state::{self, default_reserved_instances, AvailableInstances},
    util::{create_ec2_instance, get_instance_ip},
};

pub async fn handle_reserve_swarms() -> Result<()> {
    let aws_instances_hashmap = get_instances_from_aws_by_swarm_tag_and_return_hash_map().await?;
    let mut state = state::STATE.lock().await;
    let mut save_state = false;

    let mut domain_names_to_delete: Vec<String> = vec![];

    if state.reserved_instances.is_none() {
        save_state = true;
        state.reserved_instances = Some(default_reserved_instances())
    }

    let reserved = state.reserved_instances.as_mut().unwrap();

    // Stale-instance cleanup across all three lists
    for list in [
        &mut reserved.second_brain_instances,
        &mut reserved.graph_mindset_instances,
        &mut reserved.available_instances,
    ] {
        list.retain(|instance| {
            if aws_instances_hashmap.contains_key(&instance.instance_id) {
                true
            } else {
                log::info!(
                    "Reserved instance with ID: {} no longer exists on AWS, removing from reserved instances",
                    instance.instance_id
                );
                domain_names_to_delete.push(instance.host.clone());
                save_state = true;
                false
            }
        });
    }

    let reserved_instances = state.reserved_instances.clone().unwrap();

    if save_state {
        put_config_file("super", &state).await;
    }
    drop(state);

    match delete_multiple_route53_records(domain_names_to_delete).await {
        Ok(_) => {}
        Err(err) => {
            log::error!("Error deleting route53 records: {}", err.to_string());
        }
    };

    // ---- Top-up loop A: second_brain ----
    let amount_second_brain = reserved_instances.minimum_second_brain
        - reserved_instances.second_brain_instances.len() as i32;

    if amount_second_brain > 0 {
        log::info!("Reserving {} second_brain instances", amount_second_brain);
        for _ in 0..amount_second_brain {
            provision_warm_instance(None).await?;
        }
    } else {
        log::info!(
            "second_brain pool is sufficient (min={}, current={})",
            reserved_instances.minimum_second_brain,
            reserved_instances.second_brain_instances.len()
        );
    }

    // ---- Top-up loop B: graph_mindset ----
    let amount_graph_mindset = reserved_instances.minimum_graph_mindset
        - reserved_instances.graph_mindset_instances.len() as i32;

    if amount_graph_mindset > 0 {
        log::info!(
            "Reserving {} graph_mindset instances",
            amount_graph_mindset
        );
        for _ in 0..amount_graph_mindset {
            provision_warm_instance(Some("graph_mindset")).await?;
        }
    } else {
        log::info!(
            "graph_mindset pool is sufficient (min={}, current={})",
            reserved_instances.minimum_graph_mindset,
            reserved_instances.graph_mindset_instances.len()
        );
    }

    Ok(())
}

/// Provision a single warm EC2 instance and push it into the appropriate typed pool.
async fn provision_warm_instance(workspace_type: Option<&str>) -> Result<()> {
    let instance_type = "m6i.xlarge".to_string();
    let admin_password = generate_random_secret(16);

    let state = state::STATE.lock().await;
    let mut anthropic_api_key: Option<String> = None;
    if let Some(anthropic_keys) = &state.anthropic_keys {
        if !anthropic_keys.is_empty() {
            anthropic_api_key = Some(anthropic_keys[0].clone());
        }
    }
    drop(state);

    let ec2_instance = create_ec2_instance(
        None,
        None,
        instance_type.clone(),
        None,
        None,
        Some(admin_password.clone()),
        anthropic_api_key.clone(),
        None,
        None,
        workspace_type.map(|s| s.to_string()),
    )
    .await?;

    sleep(Duration::from_secs(40)).await;

    let host = format!("swarm{}.sphinx.chat", &ec2_instance.swarm_number);
    let domain_names: Vec<String> = vec![host.clone()];
    let ec2_ip_address = get_instance_ip(&ec2_instance.ec2_instance_id).await?;

    let _ = add_domain_name_to_route53(domain_names.clone(), &ec2_ip_address).await?;

    let mut state = state::STATE.lock().await;

    if let Some(used_anthropic_key) = anthropic_api_key {
        if state.anthropic_keys.clone().unwrap_or_default().len() > 1 {
            state
                .anthropic_keys
                .as_mut()
                .unwrap()
                .retain(|key| key != &used_anthropic_key);
        }
    }

    let new_instance = AvailableInstances {
        instance_id: ec2_instance.ec2_instance_id.clone(),
        instance_type: instance_type.clone(),
        swarm_number: ec2_instance.swarm_number.to_string(),
        default_host: format!("swarm{}.sphinx.chat:8800", ec2_instance.swarm_number),
        user: None,
        pass: None,
        ip_address: Some(ec2_ip_address),
        host,
        x_api_key: ec2_instance.x_api_key,
        admin_password,
    };

    let reserved = state.reserved_instances.as_mut().unwrap();
    if workspace_type == Some("graph_mindset") {
        reserved.graph_mindset_instances.push(new_instance);
    } else {
        reserved.second_brain_instances.push(new_instance);
    }

    put_config_file("super", &state).await;
    drop(state);

    Ok(())
}
