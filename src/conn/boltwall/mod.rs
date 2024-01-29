use crate::images::boltwall::BoltwallImage;
use crate::utils::docker_domain;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetAdminPubkeyBody {
    pub pubkey: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddUserBody {
    pubkey: String,
    role: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatePaidEndpointBody {
    id: u64,
    status: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoltwallAccessibility {
    is_public: bool,
}

pub async fn add_admin_pubkey(img: &BoltwallImage, pubkey: &str) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/set_admin_pubkey", host, img.port);

    let body = SetAdminPubkeyBody {
        pubkey: pubkey.to_string(),
    };
    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_super_admin(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");

    let host = docker_domain(&img.name);
    let route = format!("http://{}:{}/super_admin", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn add_user(img: &BoltwallImage, pubkey: &str, role: u32) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/set_user_role", host, img.port);

    let body = AddUserBody {
        pubkey: pubkey.to_string(),
        role: role,
    };
    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn list_admins(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/admins", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn delete_sub_admin(img: &BoltwallImage, pubkey: &str) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/sub_admin/{}", host, img.port, pubkey);

    let response = client
        .delete(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn list_paid_endpoint(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/endpointsList", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn update_paid_endpoint(img: &BoltwallImage, id: u64, status: bool) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/updateEndpointStatus", host, img.port);

    let body = UpdatePaidEndpointBody {
        id: id,
        status: status,
    };
    let response = client
        .put(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn update_boltwall_accessibility(img: &BoltwallImage, is_public: bool) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/setPublicPrivate", host, img.port);

    let body = UpdateBoltwallAccessibility { is_public };
    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_boltwall_accessibility(img: &BoltwallImage) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client");
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/getPublicPrivate", host, img.port);

    let response = client.get(route.as_str()).send().await?;

    let response_text = response.text().await?;

    Ok(response_text)
}
