use crate::{cmd::SuperSwarmResponse, state::Super};
use sphinx_swarm::config::UpdateChildSwarmPublicIpBody;

/// Info needed to decide whether/how to call Route53 (computed under read lock).
pub struct Route53UpdateInfo {
    pub domains: Vec<String>,
}

/// Phase 1: Read state to determine what Route53 domains need updating (read lock, fast).
/// Returns None if no Route53 update is needed (same IP or first registration).
pub fn get_public_ip_route53_info(
    state: &Super,
    info: &UpdateChildSwarmPublicIpBody,
) -> Option<Route53UpdateInfo> {
    let swarm_id = info.id.as_ref()?;

    // Check reserved instances first
    if let Some(reserved_instances) = &state.reserved_instances {
        if let Some(instance) = reserved_instances
            .available_instances
            .iter()
            .find(|i| format!("swarm{}", i.swarm_number) == *swarm_id)
        {
            if instance.ip_address.as_deref() == Some(&info.public_ip) {
                return None; // same IP, no update needed
            }
            if instance.ip_address.is_some() {
                return Some(Route53UpdateInfo {
                    domains: vec![instance.host.clone()],
                });
            }
            return None; // first registration, no Route53 update
        }
    }

    // Check regular stacks
    if let Some(swarm) = state.stacks.iter().find(|s| s.id == info.id) {
        if swarm.public_ip_address.as_deref() == Some(&info.public_ip) {
            return None; // same IP
        }
        if swarm.public_ip_address.is_some() {
            let mut domains = vec![swarm.host.clone()];
            if !swarm.default_host.contains(":8800") {
                domains.push(swarm.default_host.clone());
            }
            return Some(Route53UpdateInfo { domains });
        }
        return None; // first registration
    }

    None // swarm not found (will be handled in apply phase)
}

/// Phase 3: Apply the IP update to state (write lock, fast, no I/O).
pub fn apply_public_ip_update(
    state: &mut Super,
    info: &UpdateChildSwarmPublicIpBody,
) -> SuperSwarmResponse {
    if info.id.is_none() {
        return SuperSwarmResponse {
            success: false,
            message: "swarm id is required".to_string(),
            data: None,
        };
    }
    let swarm_id = info.id.clone().unwrap();

    // Check reserved instances
    if let Some(reserved_instances) = &mut state.reserved_instances {
        if let Some(pos) = reserved_instances
            .available_instances
            .iter()
            .position(|instance| format!("swarm{}", instance.swarm_number) == swarm_id)
        {
            reserved_instances.available_instances[pos].ip_address =
                Some(info.public_ip.clone());
            return SuperSwarmResponse {
                success: true,
                message: "Swarm public IP updated successfully".to_string(),
                data: None,
            };
        }
    }

    // Check regular stacks
    match state.stacks.iter().position(|swarm| swarm.id == info.id) {
        Some(swarm_pos) => {
            state.stacks[swarm_pos].public_ip_address = Some(info.public_ip.clone());
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
