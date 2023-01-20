use anyhow::Result;
use rocket::tokio;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::images::relay::RelayImage;

pub struct RelayAPI {
    pub client: reqwest::Client,
    pub url: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Users {
    pub users: Vec<User>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub alias: String,
    pub public_key: String,
    pub route_hint: String,
    pub photo_url: String,
    pub contact_key: String,
    pub person_uuid: Option<String>,
    pub is_admin: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaimBody {
    pub pubkey: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClaimRes {
    pub id: u32,
}

impl RelayAPI {
    pub async fn new(relay: &RelayImage, check_is_setup: bool) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build proxy reqwest client");
        let api = Self {
            url: format!("localhost:{}", relay.port),
            client,
            token: "".to_string(),
        };
        if !check_is_setup {
            return Ok(api);
        }
        for _ in 0..10 {
            if let Ok(_) = api.is_setup().await {
                return Ok(api);
            }
            log::info!("checking for relay setup...");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        Err(anyhow::anyhow!("relay api could not set up!"))
    }

    pub async fn is_setup(&self) -> Result<bool> {
        let route = format!("http://{}/is_setup", self.url);
        let res = self.client.get(route.as_str()).send().await?;
        Ok(res.json().await?)
    }

    pub async fn has_admin(&self) -> Result<bool> {
        let route = format!("http://{}/has_admin", self.url);
        let res = self.client.get(route.as_str()).send().await?;
        Ok(res.json().await?)
    }

    pub async fn initial_admin_pubkey(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct InitialPubkeyResult {
            pubkey: String,
        }
        let route = format!("http://{}/initial_admin_pubkey", self.url);
        let res = self.client.get(route.as_str()).send().await?;
        let ipr: InitialPubkeyResult = res.json().await?;
        Ok(ipr.pubkey)
    }

    pub async fn add_default_tribe(&self, id: u16) -> Result<bool> {
        let route = format!("http://{}/default_tribe/{}", self.url, id);
        let res = self.client.post(route.as_str()).send().await?;
        Ok(res.json().await?)
    }

    pub async fn remove_default_tribe(&self, id: u16) -> Result<bool> {
        let route = format!("http://{}/default_tribe/{}", self.url, id);
        let res = self.client.delete(route.as_str()).send().await?;
        Ok(res.json().await?)
    }

    pub async fn add_user(&self, initial_sats: Option<u64>) -> Result<User> {
        let mut sats = "".to_string();
        if let Some(s) = initial_sats {
            sats = format!("?sats={}", s);
        }
        let route = format!("http://{}/add_user{}", self.url, sats);
        let res = self
            .client
            .get(route.as_str())
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;
        Ok(res.json().await?)
    }

    pub async fn list_users(&self) -> Result<Users> {
        let route = format!("http://{}/list_users", self.url);
        let res = self
            .client
            .get(route.as_str())
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;
        Ok(res.json().await?)
    }

    pub async fn claim_user(&self, pubkey: &str, token: &str) -> Result<ClaimRes> {
        let route = format!("http://{}/contacts/tokens", self.url);
        let body = ClaimBody {
            pubkey: pubkey.to_string(),
            token: token.to_string(),
        };
        let res = self
            .client
            .post(route.as_str())
            .json(&body)
            .header("x-user-token", self.token.clone())
            .send()
            .await?;
        Ok(res.json().await?)
    }
}
