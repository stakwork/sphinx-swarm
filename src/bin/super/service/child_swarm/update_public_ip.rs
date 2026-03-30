use crate::{cmd::SuperSwarmResponse, route53::add_domain_name_to_route53, state::Super};
use sphinx_swarm::config::UpdateChildSwarmPublicIpBody;

pub async fn handle_update_child_swarm_public_ip(
    state: &mut Super,
    must_save_stack: &mut bool,
    info: UpdateChildSwarmPublicIpBody,
) -> SuperSwarmResponse {
    // TODO: implement the logic to update child swarm public IP
    if info.id.is_none() {
        return SuperSwarmResponse {
            success: false,
            message: "swarm id is required".to_string(),
            data: None,
        };
    }
    let swarm_id = info.id.clone().unwrap();
    if state.reserved_instances.is_some() {
        if let Some(reserved_instances) = &mut state.reserved_instances {
            // Find the instance in second_brain_instances, graph_mindset_instances, or legacy available_instances
            enum PoolKind { SecondBrain, GraphMindset, Legacy }
            let found: Option<(PoolKind, usize)> = reserved_instances
                .second_brain_instances
                .iter()
                .position(|i| format!("swarm{}", i.swarm_number) == swarm_id)
                .map(|p| (PoolKind::SecondBrain, p))
                .or_else(|| {
                    reserved_instances
                        .graph_mindset_instances
                        .iter()
                        .position(|i| format!("swarm{}", i.swarm_number) == swarm_id)
                        .map(|p| (PoolKind::GraphMindset, p))
                })
                .or_else(|| {
                    reserved_instances
                        .available_instances
                        .iter()
                        .position(|i| format!("swarm{}", i.swarm_number) == swarm_id)
                        .map(|p| (PoolKind::Legacy, p))
                });

            if let Some((pool_kind, pos)) = found {
                let selected_instance = match pool_kind {
                    PoolKind::SecondBrain => reserved_instances.second_brain_instances[pos].clone(),
                    PoolKind::GraphMindset => reserved_instances.graph_mindset_instances[pos].clone(),
                    PoolKind::Legacy => reserved_instances.available_instances[pos].clone(),
                };

                if selected_instance.ip_address.as_deref() == Some(&info.public_ip) {
                    return SuperSwarmResponse {
                        success: true,
                        message: "Public IP is the same as the current one, no update needed"
                            .to_string(),
                        data: None,
                    };
                }

                // Only update Route53 if the IP was previously set (not the first time)
                if selected_instance.ip_address.is_some() {
                    match add_domain_name_to_route53(
                        vec![selected_instance.host.clone()],
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

                let updated_instance = {
                    let mut inst = selected_instance;
                    inst.ip_address = Some(info.public_ip.clone());
                    inst
                };
                match pool_kind {
                    PoolKind::SecondBrain => reserved_instances.second_brain_instances[pos] = updated_instance,
                    PoolKind::GraphMindset => reserved_instances.graph_mindset_instances[pos] = updated_instance,
                    PoolKind::Legacy => reserved_instances.available_instances[pos] = updated_instance,
                }

                *must_save_stack = true;
                return SuperSwarmResponse {
                    success: true,
                    message: "Swarm public IP updated successfully".to_string(),
                    data: None,
                };
            }
        }
    }
    match state.stacks.iter().position(|swarm| swarm.id == info.id) {
        Some(swarm_pos) => {
            let selected_swarm = &mut state.stacks[swarm_pos];

            if selected_swarm.public_ip_address.as_deref() == Some(&info.public_ip) {
                return SuperSwarmResponse {
                    success: true,
                    message: "Public IP is the same as the current one, no update needed"
                        .to_string(),
                    data: None,
                };
            }

            // Only update Route53 if the IP was previously set (not the first time)
            if selected_swarm.public_ip_address.is_some() {
                let mut domains = vec![selected_swarm.host.clone()];
                if !selected_swarm.default_host.contains(":8800") {
                    domains.push(selected_swarm.default_host.clone());
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

            selected_swarm.public_ip_address = Some(info.public_ip.clone());
            *must_save_stack = true;
            return SuperSwarmResponse {
                success: true,
                message: "Swarm public IP updated successfully".to_string(),
                data: None,
            };
        }
        None => {
            return SuperSwarmResponse {
                success: false,
                message: "Swarm not found".to_string(),
                data: None,
            };
        }
    }
}
