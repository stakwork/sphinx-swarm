use crate::{cmd::SuperSwarmResponse, route53::add_domain_name_to_route53, state::state_read, state::state_write};
use sphinx_swarm::config::UpdateChildSwarmPublicIpBody;

pub fn build_route53_domains(
    route53_domain_names: Option<Vec<String>>,
    host: &str,
) -> Vec<String> {
    match route53_domain_names {
        Some(names) if !names.is_empty() => names,
        _ => vec![host.to_string()], // fallback for legacy/missing state
    }
}

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
                s.route53_domain_names.clone(),
            ))
    })
    .await;

    match stack_info {
        Some((current_ip, host, _default_host, route53_domain_names)) => {
            if current_ip.as_deref() == Some(&info.public_ip) {
                return SuperSwarmResponse {
                    success: true,
                    message: "Public IP is the same as the current one, no update needed"
                        .to_string(),
                    data: None,
                };
            }

            let domains = build_route53_domains(route53_domain_names, &host);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uses_route53_domain_names_when_present() {
        let domains = build_route53_domains(
            Some(vec!["*.swarm1.sphinx.chat".into(), "swarm1.sphinx.chat".into()]),
            "swarm1.sphinx.chat",
        );
        assert_eq!(domains, vec!["*.swarm1.sphinx.chat", "swarm1.sphinx.chat"]);
    }

    #[test]
    fn test_uses_vanity_domain_from_route53_names() {
        let domains = build_route53_domains(
            Some(vec!["custom.sphinx.chat".into()]),
            "swarm2.sphinx.chat",
        );
        assert_eq!(domains, vec!["custom.sphinx.chat"]);
    }

    #[test]
    fn test_falls_back_to_host_when_route53_names_is_none() {
        let domains = build_route53_domains(None, "swarm3.sphinx.chat");
        assert_eq!(domains, vec!["swarm3.sphinx.chat"]);
    }

    #[test]
    fn test_falls_back_to_host_when_route53_names_is_empty() {
        let domains = build_route53_domains(Some(vec![]), "swarm4.sphinx.chat");
        assert_eq!(domains, vec!["swarm4.sphinx.chat"]);
    }
}
