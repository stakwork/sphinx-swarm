use anyhow::{Context, Error, Result};
use reqwest::Client;

use crate::config::STATE;

pub async fn handle_check_public_ip_via_cron() -> Result<(), Error> {
    let current_ip = get_public_ip().await?;
    let mut state = STATE.lock().await;
    let mut alert_super_admin = false;
    if state.stack.ip.is_none() {
        alert_super_admin = true;
        state.stack.ip = Some(current_ip.clone());
    } else {
        if state.stack.ip.clone().unwrap() != current_ip {
            alert_super_admin = true;
            state.stack.ip = Some(current_ip.clone());
        }
    }
    drop(state);

    if alert_super_admin {
        log::info!("Public IP has changed to {}", current_ip);
        // updating super admin of the change
    }
    Ok(())
}

async fn get_public_ip() -> Result<String, Error> {
    let client = Client::new();

    let token = client
        .put("http://169.254.169.254/latest/api/token")
        .header("X-aws-ec2-metadata-token-ttl-seconds", "21600")
        .send()
        .await
        .context("failed to request IMDSv2 token")?
        .text()
        .await
        .context("failed to read IMDSv2 token")?;

    let ip = client
        .get("http://169.254.169.254/latest/meta-data/public-ipv4")
        .header("X-aws-ec2-metadata-token", token)
        .send()
        .await
        .context("failed to request public-ipv4")?
        .text()
        .await
        .context("failed to read public-ipv4")?
        .trim()
        .to_string();

    if ip.is_empty() {
        log::error!("instance has no public IPv4");
        return Err(anyhow::anyhow!("instance has no public IPv4"));
    }

    Ok(ip)
}
