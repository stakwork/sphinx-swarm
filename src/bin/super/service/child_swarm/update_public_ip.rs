use crate::{cmd::SuperSwarmResponse, route53::add_domain_name_to_route53, state::state_read, state::state_write};
use sphinx_swarm::config::UpdateChildSwarmPublicIpBody;

pub async fn handle_update_child_swarm_public_ip(
    proj: &str,
    info: UpdateChildSwarmPublicIpBody,
) -> SuperSwarmResponse {
    if info.id.is_none() {
        return SuperSwarmResponse {
            success: false,
            message: "swarm id is required".to_string(),
            data: None,
        };
    }
    let swarm_id = info.id.clone().unwrap();

    // Read: check if this is a reserved instance and get current state
    let reserved_info = state_read(|state| {
        if let Some(reserved_instances) = &state.reserved_instances {
            if let Some(instance) = reserved_instances
                .available_instances
                .iter()
                .find(|instance| format!("swarm{}", instance.swarm_number) == swarm_id)
            {
                return Some((
                    instance.ip_address.clone(),
                    instance.host.clone(),
                ));
            }
        }
        None
    })
    .await;

    if let Some((current_ip, host)) = reserved_info {
        if current_ip.as_deref() == Some(&info.public_ip) {
            return SuperSwarmResponse {
                success: true,
                message: "Public IP is the same as the current one, no update needed"
                    .to_string(),
                data: None,
            };
        }

        // Only update Route53 if the IP was previously set (not the first time)
        if current_ip.is_some() {
            match add_domain_name_to_route53(
                vec![host.clone()],
                &info.public_ip,
            )
            .await
            {
                Ok(_) => {
                    log::info!(
                        "Successfully updated Route53 record for swarm {} with new IP {}",
                        swarm_id,
                        info.public_ip
                    );
                }
                Err(err) => {
                    let message = format!(
                        "Failed to update Route53 record for swarm {}: {}",
                        swarm_id, err
                    );
                    log::error!("{}", message);
                    return SuperSwarmResponse {
                        success: false,
                        message,
                        data: None,
                    };
                }
            }
        } else {
            log::info!(
                "First IP registration for swarm {}: {}, skipping Route53 update",
                swarm_id,
                info.public_ip
            );
        }

        // Write: update reserved instance IP
        let public_ip = info.public_ip.clone();
        let sid = swarm_id.clone();
        state_write(proj, |state| {
            if let Some(reserved_instances) = &mut state.reserved_instances {
                if let Some(instance) = reserved_instances
                    .available_instances
                    .iter_mut()
                    .find(|i| format!("swarm{}", i.swarm_number) == sid)
                {
                    instance.ip_address = Some(public_ip);
                }
            }
        })
        .await;

        return SuperSwarmResponse {
            success: true,
            message: "Swarm public IP updated successfully".to_string(),
            data: None,
        };
    }

    // Not a reserved instance — check active stacks
    let stack_info = state_read(|state| {
        state
            .stacks
            .iter()
            .find(|swarm| swarm.id == info.id)
            .map(|s| (
                s.public_ip_address.clone(),
                s.host.clone(),
                s.default_host.clone(),
            ))
    })
    .await;

    match stack_info {
        Some((current_ip, host, default_host)) => {
            if current_ip.as_deref() == Some(&info.public_ip) {
                return SuperSwarmResponse {
                    success: true,
                    message: "Public IP is the same as the current one, no update needed"
                        .to_string(),
                    data: None,
                };
            }

            // Only update Route53 if the IP was previously set (not the first time)
            if current_ip.is_some() {
                let mut domains = vec![host.clone()];
                if !default_host.contains(":8800") {
                    domains.push(default_host.clone());
                }
                match add_domain_name_to_route53(domains, &info.public_ip).await {
                    Ok(_) => {
                        log::info!(
                            "Successfully updated Route53 record for swarm {} with new IP {}",
                            swarm_id,
                            info.public_ip
                        );
                    }
                    Err(err) => {
                        let message = format!(
                            "Failed to update Route53 record for swarm {}: {}",
                            swarm_id, err
                        );
                        log::error!("{}", message);
                        return SuperSwarmResponse {
                            success: false,
                            message,
                            data: None,
                        };
                    }
                }
            } else {
                log::info!(
                    "First IP registration for swarm {}: {}, skipping Route53 update",
                    swarm_id,
                    info.public_ip
                );
            }

            // Write: update stack IP (re-find by stable ID)
            let public_ip = info.public_ip.clone();
            let swarm_id_for_write = info.id.clone();
            state_write(proj, |state| {
                if let Some(s) = state.stacks.iter_mut().find(|s| s.id == swarm_id_for_write) {
                    s.public_ip_address = Some(public_ip);
                }
            })
            .await;

            SuperSwarmResponse {
                success: true,
                message: "Swarm public IP updated successfully".to_string(),
                data: None,
            }
        }
        None => SuperSwarmResponse {
            success: false,
            message: "Swarm not found".to_string(),
            data: None,
        },
    }
}
