pub mod setup;

use crate::images::relay::RelayImage;
use crate::utils::docker_domain;
use anyhow::{anyhow, Result};
use rocket::tokio;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct RelayAPI {
    pub client: reqwest::Client,
    pub url: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelayRes<T> {
    pub success: bool,
    pub response: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> RelayRes<T> {
    pub fn to_string(&self) -> Result<String> {
        if let Some(r) = &self.response {
            Ok(serde_json::to_string::<T>(r)?)
        } else if let Some(e) = &self.error {
            Err(anyhow!("{:?}", e))
        } else {
            if self.success {
                Ok(serde_json::to_string(&true)?)
            } else {
                Err(anyhow!("failed"))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Users {
    pub users: Vec<User>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: u32,
    pub public_key: Option<String>,
    pub deleted: Option<u8>,
    pub created_at: Option<String>,
    pub alias: Option<String>,
    pub route_hint: Option<String>,
    pub photo_url: Option<String>,
    pub contact_key: Option<String>,
    pub person_uuid: Option<String>,
    pub is_admin: Option<bool>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chat {
    id: u16,
    uuid: Option<String>,
    name: Option<String>,
    photo_url: Option<String>,
    r#type: Option<u16>,
    group_key: Option<String>,
    host: Option<String>,
    price_to_join: Option<u64>,
    price_per_message: Option<u64>,
    escrow_amount: Option<u64>,
    escrow_millis: Option<u64>,
    private: Option<u8>,
    app_url: Option<String>,
    feed_url: Option<String>,
    tenant: Option<u16>,
    pin: Option<String>,
    default_join: Option<u8>,
}
pub type ChatsRes = Vec<Chat>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTribe {
    name: String,
    is_tribe: bool,
    unlisted: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetBalance {
    reserve: u128,
    full_balance: u128,
    balance: u128,
    pending_open_balance: u128,
}

impl Default for CreateTribe {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            is_tribe: true,
            unlisted: true,
        }
    }
}

impl RelayAPI {
    pub async fn new(relay: &RelayImage, token: &str, check_is_setup: bool) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build proxy reqwest client");
        let host = docker_domain(&relay.name);
        let api = Self {
            url: format!("{}:{}", host, relay.port),
            client,
            token: token.to_string(),
        };
        if !check_is_setup {
            return Ok(api);
        }
        for i in 0..15 {
            if let Ok(_) = api.is_setup().await {
                return Ok(api);
            }
            log::info!("checking for relay setup... {}", i);
            sleep_ms(2000).await;
        }
        Err(anyhow::anyhow!("relay api could not set up!"))
    }

    pub async fn is_setup(&self) -> Result<RelayRes<bool>> {
        let route = format!("http://{}/is_setup", self.url);
        let res = self.client.get(route.as_str()).send().await?;
        Ok(res.json().await?)
    }

    pub async fn try_has_admin(&self) -> Result<RelayRes<bool>> {
        let mut err = anyhow!("try_has_admin never started");
        for _ in 0..60 {
            match self.has_admin().await {
                Ok(b) => return Ok(b),
                Err(e) => err = e,
            }
            sleep_ms(500).await;
        }
        Err(anyhow!(format!("try_has_admin ERROR: {}", err)))
    }

    pub async fn has_admin(&self) -> Result<RelayRes<bool>> {
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
        let ipr: RelayRes<InitialPubkeyResult> = res.json().await?;
        Ok(ipr.response.unwrap().pubkey)
    }

    pub async fn claim_user(&self, pubkey: &str, token: &str) -> Result<RelayRes<ClaimRes>> {
        let route = format!("http://{}/contacts/tokens", self.url);
        let body = ClaimBody {
            pubkey: pubkey.to_string(),
            token: token.to_string(),
        };
        let res = self.client.post(route.as_str()).json(&body).send().await?;
        Ok(res.json().await?)
    }

    pub async fn add_default_tribe(&self, id: u16) -> Result<RelayRes<bool>> {
        let route = format!("http://{}/default_tribe/{}", self.url, id);
        let res = self
            .client
            .post(route.as_str())
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;
        Ok(res.json().await?)
    }

    pub async fn remove_default_tribe(&self, id: u16) -> Result<RelayRes<bool>> {
        let route = format!("http://{}/default_tribe/{}", self.url, id);
        let res = self
            .client
            .delete(route.as_str())
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;
        Ok(res.json().await?)
    }

    pub async fn add_user(&self, initial_sats: Option<u64>) -> Result<RelayRes<User>> {
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
        // let hm = res.text().await?;
        // println!("ADDED USER {:?}", &hm);
        // Ok(serde_json::from_str(&hm)?)
        Ok(res.json().await?)
    }

    pub async fn list_users(&self) -> Result<RelayRes<Users>> {
        let route = format!("http://{}/list_users", self.url);
        let res = self
            .client
            .get(route.as_str())
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;
        // let hm = res.text().await?;
        // println!("HM {:?}", &hm);
        // Ok(serde_json::from_str(&hm)?)
        Ok(res.json().await?)
    }

    pub async fn get_chats(&self) -> Result<RelayRes<ChatsRes>> {
        let route = format!("http://{}/chats", self.url);
        let res = self
            .client
            .get(route.as_str())
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;
        // let hm = res.text().await?;
        // println!("get_chats -> {:?}", &hm);
        // Ok(serde_json::from_str(&hm)?)
        Ok(res.json().await?)
    }

    pub async fn create_tribe(&self, name: &str) -> Result<RelayRes<Chat>> {
        let ct = CreateTribe {
            name: name.to_string(),
            ..Default::default()
        };
        let route = format!("http://{}/group", self.url);
        let res = self
            .client
            .post(route.as_str())
            .json(&ct)
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;
        // let hm = res.text().await?;
        // println!("CREATED CHAT {:?}", &hm);
        // Ok(serde_json::from_str(&hm)?)
        Ok(res.json().await?)
    }

    pub async fn get_balance(&self) -> Result<RelayRes<GetBalance>> {
        let route = format!("http://{}/balance", self.url);
        let res = self
            .client
            .get(route.as_str())
            .header("x-admin-token", self.token.clone())
            .send()
            .await?;

        Ok(res.json().await?)
    }
}

pub async fn sleep_ms(n: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(n)).await;
}
