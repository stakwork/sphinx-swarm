use anyhow::{ Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSwarmBody {
    pub password: String,
}

pub async fn update_swarm() -> Result<String> {
    let password = std::env::var("SWARM_UPDATER_PASSWORD").unwrap_or(String::new());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build swarm updater reqwest client");

    let route = format!("http://localhost:3003/restart");

    let body = UpdateSwarmBody {
        password: password.to_string(),
    };
    let response = client
        .post(route.as_str())
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}