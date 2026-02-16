use anyhow::{anyhow, Context, Error, Result};
use reqwest::Client;

use crate::{
    config::{self, UpdateChildSwarmPublicIpBody, STATE},
    utils::{getenv, make_reqwest_client},
};

pub async fn handle_check_public_ip_via_cron(proj: &str) -> Result<(), Error> {
    let current_ip = get_public_ip().await?;
    let state = STATE.lock().await;
    let mut alert_super_admin = false;
    if state.stack.ip.is_none() {
        alert_super_admin = true;
    } else {
        if state.stack.ip.clone().unwrap() != current_ip {
            alert_super_admin = true;
        }
    }
    drop(state);

    if alert_super_admin {
        log::info!("Public IP has changed to {}", current_ip);
        // updating super admin of the change
        if let Err(e) = update_super_admin_of_ip_change(&current_ip).await {
            log::error!("Failed to notify super admin of IP change: {:?}", e);
        } else {
            log::info!("Successfully notified super admin of IP change");
            let mut state = STATE.lock().await;
            state.stack.ip = Some(current_ip);
            config::put_config_file(proj, &state.stack).await;
        }
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

async fn update_super_admin_of_ip_change(new_ip: &str) -> Result<(), Error> {
    let error_msg = "is not set in the environment variable for setting up superadmin";

    //get x-super-token
    let super_token = getenv("SUPER_TOKEN").unwrap_or("".to_string());

    if super_token.is_empty() {
        let msg = format!("SUPER_TOKEN {}", error_msg);
        log::error!("{}", msg);
        return Err(anyhow::anyhow!(
            "Failed to notify super admin of IP change: {}",
            msg
        ));
    }

    //get super url
    let super_url = getenv("SUPER_URL").unwrap_or("".to_string());

    if super_url.is_empty() {
        let msg = format!("SUPER_URL {}", error_msg);
        log::error!("{}", msg);
        return Err(anyhow::anyhow!(
            "Failed to notify super admin of IP change: {}",
            msg
        ));
    }

    let swarm_number = getenv("SWARM_NUMBER").unwrap_or("".to_string());

    if swarm_number.is_empty() {
        let msg = format!("SWARM_NUMBER {}", error_msg);
        log::error!("{}", msg);
        return Err(anyhow::anyhow!(
            "Failed to notify super admin of IP change: {}",
            msg
        ));
    }

    let client = make_reqwest_client();

    let route = format!("{}/super/update_child_public_ip", super_url);

    let body = UpdateChildSwarmPublicIpBody {
        id: Some(format!("swarm{}", swarm_number)),
        public_ip: new_ip.to_string(),
        token: None,
    };

    match client
        .post(route.as_str())
        .header("x-super-token", super_token)
        .json(&body)
        .send()
        .await
    {
        Ok(res) => {
            if res.status().is_success() {
                return Ok(());
            } else {
                let status = res.status().clone();
                let err_text = res
                    .text()
                    .await
                    .unwrap_or("Failed to read error message".to_string());
                let err_msg = format!(
                    "Failed to send new IP address to super admin: status {}, error: {}",
                    status, err_text
                );
                log::error!("{}", err_msg);
                return Err(anyhow!("{}", err_msg));
            }
        }
        Err(err) => {
            log::error!("Error sending new ip address to admin: {:?}", err);
            return Err(anyhow!(
                "Failed to notify super admin of IP change: {:?}",
                err
            ));
        }
    }
}
