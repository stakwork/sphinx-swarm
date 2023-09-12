use crate::images::cln::{hsmd_broker_ports, ClnImage};
use crate::utils::docker_domain;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

pub struct HsmdClient {
    pub client: reqwest::Client,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Connections {
    pub pubkey: Option<String>,
    pub clients: HashMap<String, bool>,
    pub current: Option<String>,
}

impl HsmdClient {
    pub async fn new(cln: &ClnImage) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("couldnt build proxy reqwest client");
        let host = docker_domain(&cln.name);
        let ps = hsmd_broker_ports(&cln.peer_port)?;
        Ok(Self {
            url: format!("{}:{}", &host, &ps.http_port),
            client,
        })
    }

    pub async fn get_clients(&self) -> Result<Connections> {
        let route = format!("http://{}/api/clients", self.url);

        let res = self.client.get(route.as_str()).send().await?;

        let conns = res.json().await?;
        println!("{:?}", conns);
        Ok(conns)
    }
}
