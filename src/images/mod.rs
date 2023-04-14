pub mod boltwall;
pub mod btc;
pub mod cache;
pub mod cln;
pub mod jarvis;
pub mod lnd;
pub mod navfiber;
pub mod neo4j;
pub mod postgres;
pub mod proxy;
pub mod relay;
pub mod traefik;

use crate::config;
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use bollard::Docker;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum Image {
    Btc(btc::BtcImage),
    Cln(cln::ClnImage),
    Lnd(lnd::LndImage),
    Relay(relay::RelayImage),
    Proxy(proxy::ProxyImage),
    Cache(cache::CacheImage),
    Neo4j(neo4j::Neo4jImage),
    NavFiber(navfiber::NavFiberImage),
    BoltWall(boltwall::BoltwallImage),
    Jarvis(jarvis::JarvisImage),
}

pub struct Repository {
    pub org: String,
    pub repo: String,
}

pub trait DockerHubImage {
    fn repo(&self) -> Repository;
}

#[async_trait]
pub trait DockerConfig {
    async fn make_config(
        &self,
        nodes: &Vec<config::Node>,
        docker: &Docker,
    ) -> Result<Config<String>>;
}

pub type Links = Vec<String>;

impl Image {
    pub fn name(&self) -> String {
        match self {
            Image::Btc(n) => n.name.clone(),
            Image::Cln(n) => n.name.clone(),
            Image::Lnd(n) => n.name.clone(),
            Image::Relay(n) => n.name.clone(),
            Image::Proxy(n) => n.name.clone(),
            Image::Cache(n) => n.name.clone(),
            Image::Neo4j(n) => n.name.clone(),
            Image::NavFiber(n) => n.name.clone(),
            Image::Jarvis(n) => n.name.clone(),
            Image::BoltWall(n) => n.name.clone(),
        }
    }
    pub fn typ(&self) -> String {
        match self {
            Image::Btc(_n) => "Btc",
            Image::Cln(_n) => "Cln",
            Image::Lnd(_n) => "Lnd",
            Image::Relay(_n) => "Relay",
            Image::Proxy(_n) => "Proxy",
            Image::Cache(_n) => "Cache",
            Image::Neo4j(_n) => "Neo4j",
            Image::NavFiber(_n) => "NavFiber",
            Image::Jarvis(_n) => "JarvisBackend",
            Image::BoltWall(_n) => "BoltWall",
        }
        .to_string()
    }
    pub fn set_version(&mut self, version: &str) {
        match self {
            Image::Btc(n) => n.version = version.to_string(),
            Image::Cln(n) => n.version = version.to_string(),
            Image::Lnd(n) => n.version = version.to_string(),
            Image::Relay(n) => n.version = version.to_string(),
            Image::Proxy(n) => n.version = version.to_string(),
            Image::Cache(n) => n.version = version.to_string(),
            Image::Neo4j(n) => n.version = version.to_string(),
            Image::NavFiber(n) => n.version = version.to_string(),
            Image::Jarvis(n) => n.version = version.to_string(),
            Image::BoltWall(n) => n.version = version.to_string(),
        };
    }
    pub async fn pre_startup(&self, docker: &Docker) -> Result<()> {
        Ok(match self {
            // unlock LND
            Image::Neo4j(n) => n.pre_startup(docker).await?,
            _ => (),
        })
    }
    pub async fn post_startup(&self, proj: &str, docker: &Docker) -> Result<()> {
        Ok(match self {
            // unlock LND
            Image::Lnd(n) => n.post_startup(proj, docker).await?,
            _ => (),
        })
    }
    pub async fn connect_client(
        &self,
        proj: &str,
        clients: &mut config::Clients,
        docker: &Docker,
        nodes: &Vec<config::Node>,
    ) -> Result<()> {
        Ok(match self {
            Image::Btc(n) => n.connect_client(clients).await,
            Image::Lnd(n) => n.connect_client(clients, docker, nodes).await?,
            Image::Cln(n) => n.connect_client(clients, docker, nodes).await?,
            Image::Proxy(n) => n.connect_client(clients).await?,
            Image::Relay(n) => n.connect_client(proj, clients).await?,
            _ => (),
        })
    }
    pub async fn post_client(&self, clients: &config::Clients) -> Result<()> {
        Ok(match self {
            // load btc wallet
            Image::Btc(n) => n.post_client(clients).await?,
            _ => (),
        })
    }
}

#[async_trait]
impl DockerConfig for Image {
    async fn make_config(
        &self,
        nodes: &Vec<config::Node>,
        docker: &Docker,
    ) -> anyhow::Result<Config<String>> {
        match self {
            Image::Btc(n) => n.make_config(nodes, docker).await,
            Image::Cln(n) => n.make_config(nodes, docker).await,
            Image::Lnd(n) => n.make_config(nodes, docker).await,
            Image::Relay(n) => n.make_config(nodes, docker).await,
            Image::Proxy(n) => n.make_config(nodes, docker).await,
            Image::Cache(n) => n.make_config(nodes, docker).await,
            Image::Neo4j(n) => n.make_config(nodes, docker).await,
            Image::NavFiber(n) => n.make_config(nodes, docker).await,
            Image::Jarvis(n) => n.make_config(nodes, docker).await,
            Image::BoltWall(n) => n.make_config(nodes, docker).await,
        }
    }
}

impl DockerHubImage for Image {
    fn repo(&self) -> Repository {
        match self {
            Image::Btc(n) => n.repo(),
            Image::Cln(n) => n.repo(),
            Image::Lnd(n) => n.repo(),
            Image::Relay(n) => n.repo(),
            Image::Proxy(n) => n.repo(),
            Image::Cache(n) => n.repo(),
            Image::Neo4j(n) => n.repo(),
            Image::NavFiber(n) => n.repo(),
            Image::Jarvis(n) => n.repo(),
            Image::BoltWall(n) => n.repo(),
        }
    }
}

pub struct LinkedImages(Vec<Image>);

// internal nodes only
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
    pub fn find_cln(&self) -> Option<cln::ClnImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_cln() {
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
    pub fn find_neo4j(&self) -> Option<neo4j::Neo4jImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_neo4j() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_jarvis(&self) -> Option<jarvis::JarvisImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_jarvis() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_boltwall(&self) -> Option<boltwall::BoltwallImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_boltwall() {
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
    pub fn as_cln(&self) -> anyhow::Result<cln::ClnImage> {
        match self {
            Image::Cln(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not CLN".to_string())),
        }
    }
    pub fn as_proxy(&self) -> anyhow::Result<proxy::ProxyImage> {
        match self {
            Image::Proxy(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Proxy".to_string())),
        }
    }
    pub fn as_neo4j(&self) -> anyhow::Result<neo4j::Neo4jImage> {
        match self {
            Image::Neo4j(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not NEO4J".to_string())),
        }
    }
    pub fn as_navfiber(&self) -> anyhow::Result<navfiber::NavFiberImage> {
        match self {
            Image::NavFiber(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not NavFiber".to_string())),
        }
    }
    pub fn as_boltwall(&self) -> anyhow::Result<boltwall::BoltwallImage> {
        match self {
            Image::BoltWall(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Boltwall".to_string())),
        }
    }
    pub fn as_jarvis(&self) -> anyhow::Result<jarvis::JarvisImage> {
        match self {
            Image::Jarvis(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Jarvis".to_string())),
        }
    }
    pub fn as_relay(&self) -> anyhow::Result<relay::RelayImage> {
        match self {
            Image::Relay(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Relay".to_string())),
        }
    }
    pub fn as_cache(&self) -> anyhow::Result<cache::CacheImage> {
        match self {
            Image::Cache(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Cache".to_string())),
        }
    }
}

fn strarr(i: Vec<&str>) -> Vec<String> {
    i.iter().map(|s| s.to_string()).collect()
}
