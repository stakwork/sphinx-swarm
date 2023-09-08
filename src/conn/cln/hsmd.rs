use crate::images::cln::ClnImage;
use crate::utils::docker_domain;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

pub struct Hsmd {
    pub client: reqwest::Client,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Clients {
    total: u128,
    balances: HashMap<String, u32>,
}

impl Hsmd {
    pub async fn new(cln: &ClnImage) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build proxy reqwest client");
        let host = docker_domain(&cln.name);
        Ok(Self {
            url: format!("{}:8010", &host),
            client
        })
    }

    pub async fn get_clients(&self) -> Result<i32> {
        let route = format!("http://{}/clients", self.url);

        let res = self
            .client
            .get(route.as_str())
            .send()
            .await?;

        let clients = res.json().await?;
        println!("{:?}",clients);
        Ok(10)
    }
}