use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::cmd::SuperSwarmResponse;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSuperAdminBody {
    pub password: String,
}

pub async fn update_super_admin() -> SuperSwarmResponse {
    let password = std::env::var("SUPER_ADMIN_UPDATER_PASSWORD").unwrap_or(String::new());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build super admin updater reqwest client");

    let route = format!("http://172.17.0.1:3003/restart-super-admin");

    let body = UpdateSuperAdminBody {
        password: password.to_string(),
    };
    let response = match client.post(route.as_str()).json(&body).send().await {
        Ok(res) => res,
        Err(err) => {
            return SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            }
        }
    };

    match response.text().await {
        Ok(text) => {
            return SuperSwarmResponse {
                success: true,
                message: text,
                data: None,
            }
        }
        Err(err) => {
            return SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            }
        }
    }
}
