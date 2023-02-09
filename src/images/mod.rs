pub mod btc;
pub mod cache;
pub mod cln_vls;
pub mod lnd;
pub mod postgres;
pub mod proxy;
pub mod relay;
pub mod traefik;
pub mod neo4j;

use crate::config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum Image {
    Btc(btc::BtcImage),
    Lnd(lnd::LndImage),
    Relay(relay::RelayImage),
    Proxy(proxy::ProxyImage),
    Cache(cache::CacheImage),
    Traefik(traefik::TraefikImage),
    Neo4j(neo4j::Neo4jImage)
}

pub struct Repository {
    pub org: String,
    pub repo: String,
}

pub trait DockerHubImage {
    fn repo(&self) -> Repository;
}

pub type Links = Vec<String>;

impl Image {
    pub fn name(&self) -> String {
        match self {
            Image::Btc(n) => n.name.clone(),
            Image::Lnd(n) => n.name.clone(),
            Image::Relay(n) => n.name.clone(),
            Image::Proxy(n) => n.name.clone(),
            Image::Cache(n) => n.name.clone(),
            Image::Traefik(n) => n.name.clone(),
            Image::Neo4j(n) => n.name.clone()
        }
    }
    pub fn typ(&self) -> String {
        match self {
            Image::Btc(_n) => "Btc",
            Image::Lnd(_n) => "Lnd",
            Image::Relay(_n) => "Relay",
            Image::Proxy(_n) => "Proxy",
            Image::Cache(_n) => "Cache",
            Image::Traefik(_n) => "Traefik",
            Image::Neo4j(n) => "Neo4j",
        }
        .to_string()
    }

    // pub fn repo(&self) -> Repository {
    //     match self {
    //         Image::Btc(n) => n.repo(),
    //         Image::Lnd(n) => n.repo(),
    //         Image::Relay(n) => n.repo(),
    //         Image::Proxy(n) => n.repo(),
    //         Image::Cache(n) => n.repo(),
    //         Image::Traefik(n) => n.repo(),
    //     }
    // }
}

impl DockerHubImage for Image {
    fn repo(&self) -> Repository {
        match self {
            Image::Btc(n) => n.repo(),
            Image::Lnd(n) => n.repo(),
            Image::Relay(n) => n.repo(),
            Image::Proxy(n) => n.repo(),
            Image::Cache(n) => n.repo(),
            Image::Traefik(n) => n.repo(),
            Image::Neo4j(n) => n.repo(),
        }
    }
}

pub struct LinkedImages(Vec<Image>);

impl LinkedImages {
    pub fn from_nodes(links: Vec<String>, nodes: &Vec<config::Node>) -> Self {
        let mut images = Vec::new();
        links.iter().for_each(|l| {
            if let Some(node) = nodes.iter().find(|n| &n.name() == l) {
                if let Ok(i) = node.as_internal() {
                    images.push(i.clone());
                }
            }
        });
        Self(images)
    }
    pub fn find_btc(&self) -> Option<btc::BtcImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_btc() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_lnd(&self) -> Option<lnd::LndImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_lnd() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_proxy(&self) -> Option<proxy::ProxyImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_proxy() {
                return Some(i);
            }
        }
        None
    }
}

impl Image {
    pub fn as_btc(&self) -> anyhow::Result<btc::BtcImage> {
        match self {
            Image::Btc(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not BTC".to_string())),
        }
    }
    pub fn as_lnd(&self) -> anyhow::Result<lnd::LndImage> {
        match self {
            Image::Lnd(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not LND".to_string())),
        }
    }
    pub fn as_proxy(&self) -> anyhow::Result<proxy::ProxyImage> {
        match self {
            Image::Proxy(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Proxy".to_string())),
        }
    }
}

fn strarr(i: Vec<&str>) -> Vec<String> {
    i.iter().map(|s| s.to_string()).collect()
}
