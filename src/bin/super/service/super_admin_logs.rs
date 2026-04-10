use sphinx_swarm::dock::{container_logs, dockr, ContainerLogsOptions};

use crate::cmd::{SuperAdminLogsRequest, SuperSwarmResponse};

pub async fn get_super_admin_docker_logs(req: SuperAdminLogsRequest) -> SuperSwarmResponse {
    let docker = dockr();
    let opts = ContainerLogsOptions {
        before_timestamp: req.before_timestamp,
        since_timestamp: req.since_timestamp,
        ..ContainerLogsOptions::default()
    };
    let logs = container_logs(&docker, "sphinx-swarm-superadmin", opts).await;
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
