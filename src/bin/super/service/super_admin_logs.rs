use sphinx_swarm::dock::{container_logs, dockr};

use crate::cmd::SuperSwarmResponse;

pub async fn get_super_admin_docker_logs() -> SuperSwarmResponse {
    let docker = dockr();
    let logs = container_logs(&docker, "sphinx-swarm-superadmin").await;
    let json_value = match serde_json::to_value(logs) {
        Ok(data) => data,
        Err(err) => {
            return SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            };
        }
    };

    return SuperSwarmResponse {
        success: true,
        message: "Super admin docker logs".to_string(),
        data: Some(json_value),
    };
}
