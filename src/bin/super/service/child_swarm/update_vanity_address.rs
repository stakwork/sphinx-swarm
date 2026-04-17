use std::collections::HashMap;

use sphinx_swarm::cmd::UpdateEnvRequest;

use crate::{
    cmd::{SuperSwarmResponse, UpdateSwarmVanityAddressInfo},
    route53::{add_domain_name_to_route53, delete_multiple_route53_records, domain_exists_in_route53},
    state::{state_read, state_write},
    util::is_valid_domain,
};

use super::update_env::handle_update_child_swarm_env_direct;

pub async fn handle_update_swarm_vanity_address(
    proj: &str,
    info: UpdateSwarmVanityAddressInfo,
) -> SuperSwarmResponse {
    let new_domain = info.vanity_address.trim().to_lowercase();

    // 1. Domain format validation
    let subdomain = match new_domain.strip_suffix(".sphinx.chat") {
        Some(s) if !s.is_empty() => s.to_string(),
        Some(_) => {
            return SuperSwarmResponse {
                success: false,
                message: "Provide a valid vanity address".to_string(),
                data: None,
            }
        }
        None => {
            return SuperSwarmResponse {
                success: false,
                message: "Vanity address must end with .sphinx.chat".to_string(),
                data: None,
            }
        }
    };

    let domain_err = is_valid_domain(subdomain);
    if !domain_err.is_empty() {
        return SuperSwarmResponse {
            success: false,
            message: domain_err,
            data: None,
        };
    }

    // 2. Route53 conflict check
    let reserved_domains = state_read(|s| s.reserved_domains.clone()).await;
    match domain_exists_in_route53(&new_domain, reserved_domains).await {
        Ok(true) => {
            return SuperSwarmResponse {
                success: false,
                message: "Domain already in use by another swarm".to_string(),
                data: None,
            }
        }
        Err(e) => {
            return SuperSwarmResponse {
                success: false,
                message: format!("Failed to check Route53: {}", e),
                data: None,
            }
        }
        Ok(false) => {}
    }

    // 3. Read swarm state
    let swarm_info = state_read(|s| {
        s.stacks
            .iter()
            .find(|sw| sw.host == info.host)
            .map(|sw| (sw.clone(), sw.public_ip_address.clone(), sw.route53_domain_names.clone()))
    })
    .await;

    let (swarm, public_ip_opt, old_route53_names) = match swarm_info {
        Some(v) => v,
        None => {
            return SuperSwarmResponse {
                success: false,
                message: format!("Swarm not found with host: {}", info.host),
                data: None,
            }
        }
    };

    let public_ip = match public_ip_opt {
        Some(ip) => ip,
        None => {
            return SuperSwarmResponse {
                success: false,
                message: "Swarm has no public IP address".to_string(),
                data: None,
            }
        }
    };

    // 4. Add new Route53 A record
    if let Err(e) = add_domain_name_to_route53(vec![new_domain.clone()], &public_ip).await {
        return SuperSwarmResponse {
            success: false,
            message: format!("Failed to add Route53 record: {}", e),
            data: None,
        };
    }

    // 5. Push env vars + restart child swarm
    let mut envs = HashMap::new();
    envs.insert("HOST".to_string(), new_domain.clone());
    envs.insert("NAV_BOLTWALL_SHARED_HOST".to_string(), new_domain.clone());

    if let Err(e) = handle_update_child_swarm_env_direct(
        &swarm,
        UpdateEnvRequest {
            id: None,
            values: envs,
        },
    )
    .await
    {
        log::error!(
            "Failed to push env vars to child swarm after Route53 record was added for domain {}. \
            Route53 record for {} pointing to {} was already added — manual remediation may be required. Error: {}",
            new_domain, new_domain, public_ip, e
        );
        return SuperSwarmResponse {
            success: false,
            message: format!("Failed to update child swarm env: {}", e),
            data: None,
        };
    }

    // 6. Delete old Route53 record (non-fatal)
    if let Some(old_names) = old_route53_names {
        if !old_names.is_empty() {
            if let Err(e) = delete_multiple_route53_records(old_names).await {
                log::warn!("Failed to delete old Route53 records for host {}: {}", info.host, e);
            }
        }
    }

    // 7. Persist state
    let nd = new_domain.clone();
    let old_host = info.host.clone();
    state_write(proj, |state| {
        if let Some(s) = state.stacks.iter_mut().find(|s| s.host == old_host) {
            s.host = nd.clone();
            s.default_host = format!("{}:8800", nd);
            s.route53_domain_names = Some(vec![nd.clone()]);
        }
    })
    .await;

    SuperSwarmResponse {
        success: true,
        message: "Vanity address updated successfully".to_string(),
        data: None,
    }
}
