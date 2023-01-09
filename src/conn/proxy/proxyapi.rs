use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};

use crate::images::ProxyImage;

pub struct ProxyAPI {
    pub client: reqwest::Client,
    pub url: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComplexBalance {
    reserve: u128,
    full_balance: u128,
    balance: u128,
    pending_open_balance: u128,
}

impl ProxyAPI {
    pub async fn new(proj: &str, proxy: &ProxyImage) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build reqwest client for proxy");
            
        Ok(Self {
            url: format!("localhost:{}", proxy.port),
            client,
            token: proxy.admin_token.clone().unwrap_or("".to_string()),
        })
    }

    pub async fn get_balance(&self) -> Result<ComplexBalance> {
        let route = format!("http://{}/balance", self.url);
        match self
            .client
            .get(route.as_str())
            .header("Content-Type", "application/json")
            .header("x-user-token", self.token.clone())
            .send()
            .await
        {
            Ok(res) => Ok(res.json().await?),
            Err(e) => Err(anyhow::anyhow!("Proxy Balance Error {:?}", e)),
        }
    }
}
