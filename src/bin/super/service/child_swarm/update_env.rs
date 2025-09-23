use anyhow::{anyhow, Error};
use sphinx_swarm::{
    cmd::{Cmd, SwarmCmd, UpdateEnvRequest},
    conn::swarm::SwarmResponse,
};

use crate::{
    cmd::{SuperSwarmResponse, UpdateChildSwarmEnvReq},
    service::update_child_swarm::handle_update_child_swarm,
    state::{RemoteStack, Super},
    util::{login_to_child_swarm, swarm_cmd},
};

pub async fn update_child_swarm_env(
    state: &Super,
    data: UpdateChildSwarmEnvReq,
) -> SuperSwarmResponse {
    let child_swarm = match state.find_swarm_by_host(&data.host, data.is_reserved) {
        Some(res) => res,
        None => {
            return SuperSwarmResponse {
                success: false,
                message: format!("Unable to find swarm with host: {}", data.host),
                data: None,
            }
        }
    };

    match handle_update_child_swarm_env(
        &child_swarm,
        UpdateEnvRequest {
            id: data.node_name,
            values: data.envs,
        },
    )
    .await
    {
        Ok(res) => res,
        Err(err) => SuperSwarmResponse {
            success: false,
            message: err.to_string(),
            data: None,
        },
    }
}

async fn handle_update_child_swarm_env(
    child_swarm: &RemoteStack,
    data: UpdateEnvRequest,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(child_swarm).await?;

    let cmd = Cmd::Swarm(SwarmCmd::UpdateEvn(data));

    let res = swarm_cmd(cmd, child_swarm.default_host.clone(), &token).await?;

    let result: SwarmResponse = match res.json().await {
        Ok(res_body) => res_body,
        Err(err) => {
            return Ok(SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            })
        }
    };

    if result.success != true {
        return Err(anyhow!(result.message));
    };

    let _ = match handle_update_child_swarm(child_swarm).await {
        Ok(_) => {}
        Err(e) => {
            log::error!(
                "Failed to update child swarm after updating env: {}",
                e.to_string()
            );
        }
    };

    Ok(SuperSwarmResponse {
        success: true,
        message: "Child swarm environment updated successfully. Restarting server now..."
            .to_string(),
        data: None,
    })
}
