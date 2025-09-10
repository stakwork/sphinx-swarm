use anyhow::Error;
use sphinx_swarm::{
    cmd::{AssignSwarmNewDetails, Cmd, SwarmCmd},
    conn::swarm::SwarmResponse,
};

use crate::{
    cmd::SuperSwarmResponse,
    state::RemoteStack,
    util::{login_to_child_swarm, swarm_cmd},
};

pub async fn call_child_swarm_to_activate_new_swarm(
    swarm_details: &RemoteStack,
    details: AssignSwarmNewDetails,
) -> Result<SuperSwarmResponse, Error> {
    let token = login_to_child_swarm(swarm_details).await?;

    let cmd = Cmd::Swarm(SwarmCmd::ChangeReservedSwarmToActive(details));
    let res = swarm_cmd(cmd, swarm_details.default_host.clone(), &token).await?;

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

    Ok(SuperSwarmResponse {
        success: result.success,
        message: result.message,
        data: None,
    })
}
