use sphinx_swarm::dock::{dockr, get_image_version};

use crate::cmd::SuperSwarmResponse;

pub async fn get_super_admin_version() -> SuperSwarmResponse {
    let docker = dockr();
    let version = get_image_version("sphinx-swarm-superadmin", &docker, "").await;
    let json_value = serde_json::to_value(version).unwrap_or_default();
    SuperSwarmResponse {
        success: true,
        message: "superadmin version".to_string(),
        data: Some(json_value),
    }
}
