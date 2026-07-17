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

    for node in &nodes {
        let cmd = Cmd::Swarm(SwarmCmd::GetEnv(node.clone()));
        let res = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            swarm_cmd(cmd, child_swarm.default_host.clone(), &token),
        )
        .await
        {
            Ok(Ok(res)) => res,
            _ => continue,
        };
        let result: SwarmResponse = match res.json().await {
            Ok(res_body) => res_body,
            Err(_) => continue,
        };
        if !result.success {
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

        return Ok(SuperSwarmResponse {
            success: true,
            message: "LLM keys retrieved".to_string(),
            data: Some(serde_json::json!({ "keys": keys })),
        });
    }

    Err(anyhow!(
        "no LLM-consuming container (bifrost/repo2graph) running — keys can't be verified remotely"
    ))
}
