use anyhow::Result;
use sphinx_swarm::utils::getenv;
use std::collections::HashMap;

use crate::cmd::SuperSwarmResponse;
use crate::state::{AvailableInstances, Super};

pub async fn nuke_single_warm_swarm(
    swarm: &AvailableInstances,
    state: &Super,
) -> Result<(SuperSwarmResponse, Option<String>)> {
    let swarm_updater_password = getenv("SWARM_UPDATER_PASSWORD")?;

    // Build env var map — same keys written by create_ec2_instance() user-data
    let mut env_vars: HashMap<String, String> = HashMap::new();

    // Preserve swarm-specific values from the AvailableInstances
    env_vars.insert("HOST".to_string(), swarm.host.clone());
    env_vars.insert("SWARM_NUMBER".to_string(), swarm.swarm_number.clone());

    // Populate from super admin environment
    let env_keys = [
        "NETWORK",
        "AWS_REGION",
        "AWS_S3_BUCKET_NAME",
        "STAKWORK_ADD_NODE_TOKEN",
        "STAKWORK_RADAR_REQUEST_TOKEN",
        "NO_REMOTE_SIGNER",
        "EXTERNAL_LND_MACAROON",
        "EXTERNAL_LND_ADDRESS",
        "EXTERNAL_LND_CERT",
        "YOUTUBE_API_TOKEN",
        "SWARM_UPDATER_PASSWORD",
        "JARVIS_FEATURE_FLAG_SCHEMA",
        "FEATURE_FLAG_TEXT_EMBEDDINGS",
        "SUPER_TOKEN",
        "SUPER_URL",
        "NAV_BOLTWALL_SHARED_HOST",
        "SECOND_BRAIN_ONLY",
        "GITHUB_PAT",
        "BOLTWALL_API_SECRET",
        "JARVIS_FEATURE_FLAG_WFA_SCHEMAS",
        "JARVIS_FEATURE_FLAG_CODEGRAPH_SCHEMAS",
        "BACKUP_KEY",
    ];

    for key in &env_keys {
        if let Ok(val) = getenv(key) {
            env_vars.insert(key.to_string(), val);
        } else if *key == "BACKUP_KEY" {
            // Always include BACKUP_KEY even if empty, matching create_ec2_instance behaviour
            env_vars.insert(key.to_string(), String::new());
        }
    }

    // PASSWORD comes from admin_password on the instance
    env_vars.insert("PASSWORD".to_string(), swarm.admin_password.clone());

    // Check port_based_ssl — broaden to accept "true" or "1"
    let port_based_ssl = getenv("PORT_BASED_SSL")
        .ok()
        .map(|v| v == "true" || v == "1");

    if port_based_ssl == Some(true) {
        env_vars.insert("PORT_BASED_SSL".to_string(), "true".to_string());
    }

    // Pull ANTHROPIC_API_KEY from the state pool (first key, same as reserve_swarm)
    let anthropic_api_key: Option<String> = state
        .anthropic_keys
        .as_ref()
        .and_then(|keys| keys.first().cloned());

    if let Some(ref key) = anthropic_api_key {
        env_vars.insert("ANTHROPIC_API_KEY".to_string(), key.clone());
    }

    let ip_address = match &swarm.ip_address {
        Some(ip) => ip.clone(),
        None => {
            return Ok((
                SuperSwarmResponse {
                    success: false,
                    message: format!("No IP address for swarm {}", swarm.host),
                    data: None,
                },
                None,
            ));
        }
    };

    let url = format!("http://{}:3003/nuke", ip_address);

    let body = serde_json::json!({
        "password": swarm_updater_password,
        "env_vars": env_vars,
        "port_based_ssl": port_based_ssl.unwrap_or(false),
    });

    let client = sphinx_swarm::utils::make_reqwest_client();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to POST /nuke to {}: {}", url, e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();

    if status.is_success() {
        log::info!("Nuke succeeded for swarm {}: {}", swarm.host, text);
        Ok((
            SuperSwarmResponse {
                success: true,
                message: format!("Nuke triggered for swarm {}", swarm.host),
                data: None,
            },
            anthropic_api_key,
        ))
    } else {
        log::error!(
            "Nuke failed for swarm {} (status {}): {}",
            swarm.host,
            status,
            text
        );
        Ok((
            SuperSwarmResponse {
                success: false,
                message: format!(
                    "Nuke failed for swarm {} (status {}): {}",
                    swarm.host, status, text
                ),
                data: None,
            },
            None,
        ))
    }
}

pub async fn nuke_warm_swarm_by_host(host: &str, state: &Super) -> (SuperSwarmResponse, Option<String>) {
    // Guard: must be in available_instances (not in state.stacks)
    let available = match &state.reserved_instances {
        Some(ri) => ri
            .available_instances
            .iter()
            .find(|i| i.host == host)
            .cloned(),
        None => None,
    };

    let swarm = match available {
        Some(s) => s,
        None => {
            return (
                SuperSwarmResponse {
                    success: false,
                    message: format!(
                        "Host '{}' is not in the warm pool. Nuke rejected.",
                        host
                    ),
                    data: None,
                },
                None,
            );
        }
    };

    match nuke_single_warm_swarm(&swarm, state).await {
        Ok((res, used_key)) => (res, used_key),
        Err(e) => (
            SuperSwarmResponse {
                success: false,
                message: format!("Nuke error for {}: {}", host, e),
                data: None,
            },
            None,
        ),
    }
}

pub async fn nuke_all_warm_swarms(state: &Super) -> (SuperSwarmResponse, Vec<String>) {
    let available = match &state.reserved_instances {
        Some(ri) => ri.available_instances.clone(),
        None => {
            return (
                SuperSwarmResponse {
                    success: false,
                    message: "No reserved instances configured".to_string(),
                    data: None,
                },
                vec![],
            );
        }
    };

    if available.is_empty() {
        return (
            SuperSwarmResponse {
                success: true,
                message: "No warm swarms to nuke".to_string(),
                data: None,
            },
            vec![],
        );
    }

    let mut errors: Vec<String> = Vec::new();
    let mut success_count = 0u32;
    let mut used_keys: Vec<String> = Vec::new();

    for swarm in &available {
        match nuke_single_warm_swarm(swarm, state).await {
            Ok((res, used_key)) => {
                if res.success {
                    success_count += 1;
                    if let Some(key) = used_key {
                        used_keys.push(key);
                    }
                } else {
                    errors.push(format!("{}: {}", swarm.host, res.message));
                }
            }
            Err(e) => {
                log::error!("Auto-nuke error for {}: {}", swarm.host, e);
                errors.push(format!("{}: {}", swarm.host, e));
            }
        }
    }

    if errors.is_empty() {
        (
            SuperSwarmResponse {
                success: true,
                message: format!("Nuked {} warm swarm(s) successfully", success_count),
                data: None,
            },
            used_keys,
        )
    } else {
        (
            SuperSwarmResponse {
                success: false,
                message: format!(
                    "Nuked {}/{} warm swarms. Errors: {}",
                    success_count,
                    available.len(),
                    errors.join("; ")
                ),
                data: None,
            },
            used_keys,
        )
    }
}
