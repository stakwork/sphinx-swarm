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
            if let Some(pos) = reserved_instances
                .available_instances
                .iter()
                .position(|instance| format!("swarm{}", instance.swarm_number) == swarm_id)
            {
                let mut selected_instance = reserved_instances.available_instances[pos].clone();

                if selected_instance.ip_address.is_some()
                    && selected_instance.ip_address.clone().unwrap() == info.public_ip
                {
                    return SuperSwarmResponse {
                        success: true,
                        message: "Public IP is the same as the current one, no update needed"
                            .to_string(),
                        data: None,
                    };
                }

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
                selected_instance.ip_address = Some(info.public_ip.clone());
                reserved_instances.available_instances[pos] = selected_instance;

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

            if selected_swarm.public_ip_address.is_some()
                && selected_swarm.public_ip_address.clone().unwrap() == info.public_ip
            {
                return SuperSwarmResponse {
                    success: true,
                    message: "Public IP is the same as the current one, no update needed"
                        .to_string(),
                    data: None,
                };
            }

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
