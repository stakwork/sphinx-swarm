use anyhow::Result;
use bollard::image;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::cmd::GetDockerImageTagsDetails;

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

    let route = format!("http://172.17.0.1:3003/restart");

    let body = UpdateSwarmBody {
        password: password.to_string(),
    };
    let response = client.post(route.as_str()).json(&body).send().await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_image_tags(image_details: GetDockerImageTagsDetails) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build swarm updater reqwest client");

    let url = format!(
        "https://hub.docker.com/v2/repositories/{}/tags?page={}&page_size={}",
        image_details.org_image_name, image_details.page, image_details.page_size
    );

    let response = client.get(url.as_str()).send().await?;

    let response_text = response.text().await?;

    Ok(response_text)
}
