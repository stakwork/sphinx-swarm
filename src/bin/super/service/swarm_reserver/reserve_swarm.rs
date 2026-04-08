use anyhow::Result;
use tokio::time::{sleep, Duration};

use crate::{
    ec2::get_instances_from_aws_by_swarm_tag_and_return_hash_map,
    put_config_file,
    route53::{add_domain_name_to_route53, delete_multiple_route53_records},
    service::swarm_reserver::{
        nuke_warm_swarm::nuke_single_warm_swarm, utils::generate_random_secret,
    },
    state::{self, default_reserved_instances, AvailableInstances, RemoteStack},
    util::{create_ec2_instance, get_child_swarm_image_versions, get_instance_ip},
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

    state.reserved_instances.as_mut().unwrap().available_instances.retain(|reserved_instance| {
        if aws_instances_hashmap.contains_key(&reserved_instance.instance_id) {
            true
        } else {
            log::info!("Reserved instance with ID: {} no longer exists on AWS, removing from reserved instances", reserved_instance.instance_id);
            domain_names_to_delete.push(reserved_instance.host.clone());
            save_state = true;
            false
        }
    });
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

    let amount_to_reserve =
        reserved_instances.minimum_available - reserved_instances.available_instances.len() as i32;

    if amount_to_reserve <= 0 {
        log::info!(
            "No need to reserve more instances at the moment, Amount tp reserver: {}",
            amount_to_reserve
        );
        check_and_auto_nuke_stale_warm_swarms().await;
        return Ok(());
    }

    log::info!("Reserving {} instances", amount_to_reserve);

    for _ in 0..amount_to_reserve {
        // TODO: decide what we want to be the default instance type: ASK GONZALO
        let instance_type = "m6i.xlarge".to_string();
        let admin_password = generate_random_secret(16);
        let state = state::STATE.lock().await;
        let mut anthropic_api_key: Option<String> = None;

        if let Some(anthropic_keys) = &state.anthropic_keys {
            if anthropic_keys.len() > 0 {
                anthropic_api_key = Some(anthropic_keys[0].clone());
            }
        }

        drop(state);

        let ec2_instance = create_ec2_instance(
            None,
            None,
            instance_type.to_string().clone(),
            None,
            None,
            Some(admin_password.clone()),
            anthropic_api_key.clone(),
            None,
            None,
            None, // workspace_type: warm swarms default to second_brain
        )
        .await?;

        sleep(Duration::from_secs(40)).await;

        let host = format!("swarm{}.sphinx.chat", &ec2_instance.swarm_number);

        let domain_names: Vec<String> = vec![host.clone()];

        let ec2_ip_address = get_instance_ip(&ec2_instance.ec2_instance_id).await?;

        let _ = add_domain_name_to_route53(domain_names.clone(), &ec2_ip_address).await?;

        let mut state = state::STATE.lock().await;

        if let Some(used_anthropic_key) = anthropic_api_key {
            if state.anthropic_keys.clone().unwrap().len() > 1 {
                state
                    .anthropic_keys
                    .as_mut()
                    .unwrap()
                    .retain(|key| key != &used_anthropic_key);
            }
        }

        state
            .reserved_instances
            .as_mut()
            .unwrap()
            .available_instances
            .push(AvailableInstances {
                instance_id: ec2_instance.ec2_instance_id.clone(),
                instance_type: instance_type.to_string(),
                swarm_number: ec2_instance.swarm_number.to_string(),
                default_host: format!(
                    "swarm{}.sphinx.chat:8800",
                    ec2_instance.swarm_number.clone()
                ),
                user: None,
                pass: None,
                ip_address: Some(ec2_ip_address),
                host,
                x_api_key: ec2_instance.x_api_key,
                admin_password,
            });

        put_config_file("super", &state).await;
        drop(state)
    }

    // After provisioning, check for stale images and auto-nuke
    check_and_auto_nuke_stale_warm_swarms().await;

    Ok(())
}

async fn check_and_auto_nuke_stale_warm_swarms() {
    let available: Vec<AvailableInstances> = {
        let state = state::STATE.lock().await;
        match &state.reserved_instances {
            Some(ri) => ri.available_instances.clone(),
            None => return,
        }
    };

    for swarm in &available {
        // Build a minimal RemoteStack so we can call get_child_swarm_image_versions
        let remote_stack = RemoteStack {
            host: swarm.host.clone(),
            default_host: swarm.default_host.clone(),
            ec2_instance_id: swarm.instance_id.clone(),
            user: swarm.user.clone(),
            pass: swarm.pass.clone(),
            public_ip_address: swarm.ip_address.clone(),
            ..Default::default()
        };

        let image_versions = match get_child_swarm_image_versions(&remote_stack).await {
            Ok(res) => res,
            Err(e) => {
                log::error!(
                    "Auto-nuke check: failed to get image versions for {}: {}",
                    swarm.host,
                    e
                );
                continue;
            }
        };

        // data is Vec<ImageVersion>; look for sphinx-swarm entry
        let is_stale = image_versions
            .data
            .as_ref()
            .and_then(|d| {
                if let serde_json::Value::Array(arr) = d {
                    arr.iter().find(|v| {
                        v.get("name")
                            .and_then(|n| n.as_str())
                            .map(|n| n.contains("sphinx-swarm"))
                            .unwrap_or(false)
                    })
                    .and_then(|v| v.get("is_latest"))
                    .and_then(|il| il.as_bool())
                    .map(|is_latest| !is_latest)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        if is_stale {
            log::info!(
                "Auto-nuke: warm swarm {} is running a stale sphinx-swarm image. Nuking...",
                swarm.host
            );
            let state = state::STATE.lock().await;
            match nuke_single_warm_swarm(swarm, &state).await {
                Ok(res) => {
                    if res.success {
                        log::info!("Auto-nuke succeeded for {}", swarm.host);
                    } else {
                        log::error!("Auto-nuke failed for {}: {}", swarm.host, res.message);
                    }
                }
                Err(e) => {
                    log::error!("Auto-nuke error for {}: {}", swarm.host, e);
                }
            }
            drop(state);
        }
    }
}
