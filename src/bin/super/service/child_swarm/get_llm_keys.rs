use anyhow::{anyhow, Error};
use std::collections::HashMap;

use sphinx_swarm::{
    cmd::{Cmd, SwarmCmd},
    conn::swarm::SwarmResponse,
};

use crate::{
    cmd::{GetChildSwarmLlmKeysReq, SuperSwarmResponse},
    state::RemoteStack,
    util::{login_to_child_swarm, mask_key, swarm_cmd},
};

pub const LLM_KEY_NAMES: [&str; 4] = [
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_API_KEY",
    "OPENROUTER_API_KEY",
];

// containers that receive the LLM keys in their env, in lookup order.
// only containers that actually consume the keys belong here — inspecting
// any other container would report "not set" even when .env has the key
const LLM_KEY_NODES: [&str; 2] = ["bifrost", "repo2graph"];

pub async fn get_child_swarm_llm_keys(
    swarm: Option<RemoteStack>,
    data: GetChildSwarmLlmKeysReq,
) -> SuperSwarmResponse {
    let child_swarm = match swarm {
        Some(res) => res,
        None => {
            return SuperSwarmResponse {
                success: false,
                message: format!("Unable to find swarm with host: {}", data.host),
                data: None,
            }
        }
    };

    match handle_get_child_swarm_llm_keys(&child_swarm, data.node_name).await {
        Ok(res) => res,
        Err(err) => SuperSwarmResponse {
            success: false,
            message: err.to_string(),
            data: None,
        },
    }
}

async fn handle_get_child_swarm_llm_keys(
    child_swarm: &RemoteStack,
    node_name: Option<String>,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(child_swarm).await?;

    let nodes: Vec<String> = match node_name {
        Some(node) if !node.is_empty() => vec![node],
        _ => LLM_KEY_NODES.iter().map(|n| n.to_string()).collect(),
    };

    let mut last_err: Option<String> = None;

    for node in &nodes {
        let cmd = Cmd::Swarm(SwarmCmd::GetEnv(node.clone()));
        let res = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            swarm_cmd(cmd, child_swarm.default_host.clone(), &token),
        )
        .await
        {
            Ok(Ok(res)) => res,
            Ok(Err(err)) => {
                last_err = Some(err.to_string());
                continue;
            }
            Err(_) => {
                last_err = Some(format!("timed out reading env from {}", node));
                continue;
            }
        };
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if status != 200 {
            let snippet: String = body.chars().take(120).collect();
            last_err = Some(format!("{} from child swarm: {}", status, snippet));
            continue;
        }
        let result: SwarmResponse = match serde_json::from_str(&body) {
            Ok(res_body) => res_body,
            Err(_) => {
                let snippet: String = body.chars().take(120).collect();
                last_err = Some(format!("unexpected response from child swarm: {}", snippet));
                continue;
            }
        };
        if !result.success {
            last_err = Some(result.message);
            continue;
        }
        let envs: HashMap<String, String> = match result.data {
            Some(data) => serde_json::from_value(data).unwrap_or_default(),
            None => continue,
        };

        let mut keys: HashMap<String, String> = HashMap::new();
        for key_name in LLM_KEY_NAMES {
            if let Some(value) = envs.get(key_name) {
                if !value.is_empty() {
                    keys.insert(key_name.to_string(), mask_key(value));
                }
            }
        }

        // `keys` keeps its shape (a swarm without an Anthropic key just has
        // no ANTHROPIC_API_KEY entry, as before); the flag says that absence
        // is by design.
        let anthropic_workload_identity = anthropic_uses_workload_identity(&envs);

        return Ok(SuperSwarmResponse {
            success: true,
            message: "LLM keys retrieved".to_string(),
            data: Some(serde_json::json!({
                "keys": keys,
                "anthropic_workload_identity": anthropic_workload_identity,
            })),
        });
    }

    Err(anyhow!(last_err.unwrap_or_else(|| {
        "no LLM-consuming container (bifrost/repo2graph) running — keys can't be verified remotely"
            .to_string()
    })))
}

/// Whether a container's env has Anthropic through workload identity
/// federation instead of a key: no ANTHROPIC_API_KEY, and the federation
/// rule, organization and service-account ids (the gateway's own rule).
fn anthropic_uses_workload_identity(envs: &HashMap<String, String>) -> bool {
    let set = |k: &str| envs.get(k).is_some_and(|v| !v.is_empty());
    !set("ANTHROPIC_API_KEY")
        && set("ANTHROPIC_FEDERATION_RULE_ID")
        && set("ANTHROPIC_ORGANIZATION_ID")
        && set("ANTHROPIC_SERVICE_ACCOUNT_ID")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    const FED: [(&str, &str); 3] = [
        ("ANTHROPIC_FEDERATION_RULE_ID", "fdrl_1"),
        ("ANTHROPIC_ORGANIZATION_ID", "org-1"),
        ("ANTHROPIC_SERVICE_ACCOUNT_ID", "svac_1"),
    ];

    #[test]
    fn workload_identity_is_no_key_plus_the_three_ids() {
        assert!(anthropic_uses_workload_identity(&envs(&FED)));
        assert!(anthropic_uses_workload_identity(&envs(&[
            FED[0],
            FED[1],
            FED[2],
            ("ANTHROPIC_API_KEY", ""),
            ("ANTHROPIC_WORKSPACE_ID", "wrkspc_1"),
        ])));
        assert!(!anthropic_uses_workload_identity(&envs(&[
            FED[0],
            FED[1],
            FED[2],
            ("ANTHROPIC_API_KEY", "sk-ant-x"),
        ])));
        assert!(!anthropic_uses_workload_identity(&envs(&FED[..2])));
        assert!(!anthropic_uses_workload_identity(&envs(&[])));
    }
}
