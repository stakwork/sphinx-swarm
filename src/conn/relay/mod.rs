use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use crate::images::RelayImage;

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
}

impl RelayAPI {
    pub async fn new(relay: &RelayImage) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build proxy reqwest client");

        Ok(Self {
            url: format!("localhost:{}", relay.port),
            client,
            token: "".to_string(),
        })
    }

    pub async fn add_user(&self) -> Result<User> {
        let route = format!("http://{}/add_user", self.url);

        let res = self
            .client
            .get(route.as_str())
            .header("x-user-token", self.token.clone())
            .send()
            .await?;

        Ok(res.json().await?)
    }

    pub async fn list_users(&self) -> Result<Users> {
        let route = format!("http://{}/list_users", self.url);

        let res = self
            .client
            .get(route.as_str())
            .header("x-user-token", self.token.clone())
            .send()
            .await?;

        Ok(res.json().await?)
    }
}
