pub mod boltwall;
pub mod bot;
pub mod broker;
pub mod btc;
pub mod builtin;
pub mod cache;
pub mod chrome;
pub mod cln;
pub mod config_server;
pub mod dufs;
pub mod elastic;
pub mod jamie;
pub mod jarvis;
pub mod llama;
pub mod lnd;
pub mod lss;
pub mod mixer;
pub mod mongo;
pub mod navfiber;
pub mod neo4j;
pub mod postgres;
pub mod proxy;
pub mod redis;
pub mod relay;
pub mod repo2graph;
pub mod rqbit;
pub mod runner;
pub mod stakgraph;
pub mod tome;
pub mod traefik;
pub mod tribes;
pub mod whisker;
pub mod whisper;

use crate::config;
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use bollard::Docker;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Image {
    Btc(btc::BtcImage),
    Cln(cln::ClnImage),
    Lnd(lnd::LndImage),
    Relay(relay::RelayImage),
    Proxy(proxy::ProxyImage),
    Cache(cache::CacheImage),
    Neo4j(neo4j::Neo4jImage),
    Elastic(elastic::ElasticImage),
    NavFiber(navfiber::NavFiberImage),
    BoltWall(boltwall::BoltwallImage),
    Jarvis(jarvis::JarvisImage),
    Lss(lss::LssImage),
    Broker(broker::BrokerImage),
    Mixer(mixer::MixerImage),
    Tribes(tribes::TribesImage),
    Config(config_server::ConfigImage),
    Bot(bot::BotImage),
    Builtin(builtin::BuiltinImage),
    Dufs(dufs::DufsImage),
    Tome(tome::TomeImage),
    Rqbit(rqbit::RqbitImage),
    Llama(llama::LlamaImage),
    Whisper(whisper::WhisperImage),
    Whisker(whisker::WhiskerImage),
    Runner(runner::RunnerImage),
    Mongo(mongo::MongoImage),
    Jamie(jamie::JamieImage),
    Repo2Graph(repo2graph::Repo2GraphImage),
    Redis(redis::RedisImage),
    Chrome(chrome::ChromeImage),
    Stakgraph(stakgraph::StakgraphImage),
}

pub enum Registry {
    DockerHub,
    Ghcr,
    Local,
}

pub struct Repository {
    pub registry: Registry,
    pub org: String,
    pub repo: String,
    pub root_volume: String,
}

pub trait DockerHubImage {
    fn repo(&self) -> Repository;
    fn image(&self) -> String {
        let repo = self.repo();
        if repo.org == "library" {
            return repo.repo.clone();
        }
        match repo.registry {
            Registry::DockerHub => format!("{}/{}", repo.org, repo.repo),
            Registry::Ghcr => format!("ghcr.io/{}/{}", repo.org, repo.repo),
            Registry::Local => format!("{}", repo.repo),
        }
    }
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
            Image::Elastic(n) => n.name.clone(),
            Image::NavFiber(n) => n.name.clone(),
            Image::Jarvis(n) => n.name.clone(),
            Image::BoltWall(n) => n.name.clone(),
            Image::Lss(n) => n.name.clone(),
            Image::Broker(n) => n.name.clone(),
            Image::Mixer(n) => n.name.clone(),
            Image::Tribes(n) => n.name.clone(),
            Image::Config(n) => n.name.clone(),
            Image::Bot(n) => n.name.clone(),
            Image::Builtin(n) => n.name.clone(),
            Image::Dufs(n) => n.name.clone(),
            Image::Tome(n) => n.name.clone(),
            Image::Rqbit(n) => n.name.clone(),
            Image::Llama(n) => n.name.clone(),
            Image::Whisper(n) => n.name.clone(),
            Image::Whisker(n) => n.name.clone(),
            Image::Runner(n) => n.name.clone(),
            Image::Mongo(n) => n.name.clone(),
            Image::Jamie(n) => n.name.clone(),
            Image::Repo2Graph(n) => n.name.clone(),
            Image::Redis(n) => n.name.clone(),
            Image::Chrome(n) => n.name.clone(),
            Image::Stakgraph(n) => n.name.clone(),
        }
    }

    pub fn host(&self) -> Option<String> {
        match self {
            Image::Btc(n) => n.host.clone(),
            Image::Cln(n) => n.host.clone(),
            Image::Lnd(n) => n.host.clone(),
            Image::Relay(n) => n.host.clone(),
            Image::Proxy(_) => None,
            Image::Cache(_) => None,
            Image::Neo4j(n) => n.host.clone(),
            Image::Elastic(n) => n.host.clone(),
            Image::NavFiber(n) => n.host.clone(),
            Image::Jarvis(_) => None,
            Image::BoltWall(n) => n.host.clone(),
            Image::Lss(_) => None,
            Image::Broker(n) => n.host.clone(),
            Image::Mixer(n) => n.host.clone(),
            Image::Tribes(n) => n.host.clone(),
            Image::Config(n) => n.host.clone(),
            Image::Bot(n) => n.host.clone(),
            Image::Builtin(n) => n.host.clone(),
            Image::Dufs(n) => n.host.clone(),
            Image::Tome(n) => n.host.clone(),
            Image::Rqbit(n) => n.host.clone(),
            Image::Llama(n) => n.host.clone(),
            Image::Whisper(n) => n.host.clone(),
            Image::Whisker(_) => None,
            Image::Runner(n) => n.host.clone(),
            Image::Mongo(n) => n.host.clone(),
            Image::Jamie(n) => n.host.clone(),
            Image::Repo2Graph(n) => n.host.clone(),
            Image::Redis(n) => n.host.clone(),
            Image::Chrome(n) => n.host.clone(),
            Image::Stakgraph(n) => n.host.clone(),
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
            Image::Elastic(_n) => "Elastic",
            Image::NavFiber(_n) => "NavFiber",
            Image::Jarvis(_n) => "JarvisBackend",
            Image::BoltWall(_n) => "BoltWall",
            Image::Lss(_n) => "LSS",
            Image::Broker(_n) => "Broker",
            Image::Mixer(_n) => "Mixer",
            Image::Tribes(_n) => "Tribes",
            Image::Config(_n) => "Config",
            Image::Bot(_n) => "Bot",
            Image::Builtin(_n) => "Builtin",
            Image::Dufs(_n) => "Dufs",
            Image::Tome(_n) => "Tome",
            Image::Rqbit(_n) => "Rqbit",
            Image::Llama(_n) => "Llama",
            Image::Whisper(_n) => "Whisper",
            Image::Whisker(_n) => "Whisker",
            Image::Runner(_n) => "Runner",
            Image::Mongo(_n) => "Mongo",
            Image::Jamie(_n) => "Jamie",
            Image::Repo2Graph(_n) => "Repo2Graph",
            Image::Redis(_n) => "Redis",
            Image::Chrome(_n) => "Chrome",
            Image::Stakgraph(_n) => "Stakgraph",
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
            Image::Elastic(n) => n.version = version.to_string(),
            Image::NavFiber(n) => n.version = version.to_string(),
            Image::Jarvis(n) => n.version = version.to_string(),
            Image::BoltWall(n) => n.version = version.to_string(),
            Image::Lss(n) => n.version = version.to_string(),
            Image::Broker(n) => n.version = version.to_string(),
            Image::Mixer(n) => n.version = version.to_string(),
            Image::Tribes(n) => n.version = version.to_string(),
            Image::Config(n) => n.version = version.to_string(),
            Image::Bot(n) => n.version = version.to_string(),
            Image::Builtin(n) => n.version = version.to_string(),
            Image::Dufs(n) => n.version = version.to_string(),
            Image::Tome(n) => n.version = version.to_string(),
            Image::Rqbit(n) => n.version = version.to_string(),
            Image::Llama(_n) => (),
            Image::Whisper(_n) => (),
            Image::Whisker(_n) => (),
            Image::Runner(n) => n.version = version.to_string(),
            Image::Mongo(n) => n.version = version.to_string(),
            Image::Jamie(n) => n.version = version.to_string(),
            Image::Repo2Graph(n) => n.version = version.to_string(),
            Image::Redis(n) => n.version = version.to_string(),
            Image::Chrome(n) => n.version = version.to_string(),
            Image::Stakgraph(n) => n.version = version.to_string(),
        }
    }

    pub fn set_host(&mut self, host: &str) {
        match self {
            Image::Btc(n) => n.host(Some(host.to_string())),
            Image::Cln(n) => n.host(Some(host.to_string())),
            Image::Lnd(n) => n.host(Some(host.to_string())),
            Image::Relay(n) => n.host(Some(host.to_string())),
            Image::Proxy(_) => (),
            Image::Cache(_) => (),
            Image::Neo4j(n) => n.host(Some(host.to_string())),
            Image::Elastic(n) => n.host(Some(host.to_string())),
            Image::NavFiber(n) => n.host(Some(host.to_string())),
            Image::Jarvis(_) => (),
            Image::BoltWall(n) => n.host(Some(host.to_string())),
            Image::Lss(_) => (),
            Image::Broker(n) => n.host(Some(host.to_string())),
            Image::Mixer(n) => n.host(Some(host.to_string())),
            Image::Tribes(n) => n.host(Some(host.to_string())),
            Image::Config(n) => n.host(Some(host.to_string())),
            Image::Bot(n) => n.host(Some(host.to_string())),
            Image::Builtin(n) => n.host(Some(host.to_string())),
            Image::Dufs(n) => n.host(Some(host.to_string())),
            Image::Tome(n) => n.host(Some(host.to_string())),
            Image::Rqbit(n) => n.host(Some(host.to_string())),
            Image::Llama(_n) => (),
            Image::Whisper(_n) => (),
            Image::Whisker(_n) => (),
            Image::Runner(n) => n.host(Some(host.to_string())),
            Image::Mongo(n) => n.host(Some(host.to_string())),
            Image::Jamie(n) => n.host(Some(host.to_string())),
            Image::Repo2Graph(n) => n.host(Some(host.to_string())),
            Image::Redis(n) => n.host(Some(host.to_string())),
            Image::Chrome(n) => n.host(Some(host.to_string())),
            Image::Stakgraph(n) => n.host(Some(host.to_string())),
        }
    }
    pub async fn pre_startup(&self, docker: &Docker) -> Result<()> {
        Ok(match self {
            Image::Cln(n) => n.pre_startup(docker).await?,
            Image::Neo4j(n) => n.pre_startup(docker).await?,
            _ => (),
        })
    }
    pub async fn post_startup(&self, proj: &str, docker: &Docker) -> Result<()> {
        Ok(match self {
            // unlock LND
            Image::Lnd(n) => n.post_startup(proj, docker).await?,
            Image::Elastic(n) => n.post_startup(proj, docker).await?,
            _ => (),
        })
    }
    pub fn remove_client(&self, clients: &mut config::Clients) {
        match self {
            Image::Btc(n) => n.remove_client(clients),
            Image::Lnd(n) => n.remove_client(clients),
            Image::Cln(n) => n.remove_client(clients),
            Image::Proxy(n) => n.remove_client(clients),
            Image::Relay(n) => n.remove_client(clients),
            _ => (),
        }
    }
    pub async fn connect_client<Canceller>(
        &self,
        proj: &str,
        clients: &mut config::Clients,
        docker: &Docker,
        nodes: &Vec<config::Node>,
        canceller: Canceller,
    ) -> Result<()>
    where
        Canceller: Fn() -> bool,
    {
        Ok(match self {
            Image::Btc(n) => n.connect_client(clients).await,
            Image::Lnd(n) => n.connect_client(clients, docker, nodes).await?,
            Image::Cln(n) => n.connect_client(clients, docker, nodes, canceller).await?,
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
            Image::Elastic(n) => n.make_config(nodes, docker).await,
            Image::NavFiber(n) => n.make_config(nodes, docker).await,
            Image::Jarvis(n) => n.make_config(nodes, docker).await,
            Image::BoltWall(n) => n.make_config(nodes, docker).await,
            Image::Lss(n) => n.make_config(nodes, docker).await,
            Image::Broker(n) => n.make_config(nodes, docker).await,
            Image::Mixer(n) => n.make_config(nodes, docker).await,
            Image::Tribes(n) => n.make_config(nodes, docker).await,
            Image::Config(n) => n.make_config(nodes, docker).await,
            Image::Bot(n) => n.make_config(nodes, docker).await,
            Image::Builtin(n) => n.make_config(nodes, docker).await,
            Image::Dufs(n) => n.make_config(nodes, docker).await,
            Image::Tome(n) => n.make_config(nodes, docker).await,
            Image::Rqbit(n) => n.make_config(nodes, docker).await,
            Image::Llama(n) => n.make_config(nodes, docker).await,
            Image::Whisper(n) => n.make_config(nodes, docker).await,
            Image::Whisker(n) => n.make_config(nodes, docker).await,
            Image::Runner(n) => n.make_config(nodes, docker).await,
            Image::Mongo(n) => n.make_config(nodes, docker).await,
            Image::Jamie(n) => n.make_config(nodes, docker).await,
            Image::Repo2Graph(n) => n.make_config(nodes, docker).await,
            Image::Redis(n) => n.make_config(nodes, docker).await,
            Image::Chrome(n) => n.make_config(nodes, docker).await,
            Image::Stakgraph(n) => n.make_config(nodes, docker).await,
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
            Image::Elastic(n) => n.repo(),
            Image::NavFiber(n) => n.repo(),
            Image::Jarvis(n) => n.repo(),
            Image::BoltWall(n) => n.repo(),
            Image::Lss(n) => n.repo(),
            Image::Broker(n) => n.repo(),
            Image::Mixer(n) => n.repo(),
            Image::Tribes(n) => n.repo(),
            Image::Config(n) => n.repo(),
            Image::Bot(n) => n.repo(),
            Image::Builtin(n) => n.repo(),
            Image::Dufs(n) => n.repo(),
            Image::Tome(n) => n.repo(),
            Image::Rqbit(n) => n.repo(),
            Image::Llama(n) => n.repo(),
            Image::Whisper(n) => n.repo(),
            Image::Whisker(n) => n.repo(),
            Image::Runner(n) => n.repo(),
            Image::Mongo(n) => n.repo(),
            Image::Jamie(n) => n.repo(),
            Image::Repo2Graph(n) => n.repo(),
            Image::Redis(n) => n.repo(),
            Image::Chrome(n) => n.repo(),
            Image::Stakgraph(n) => n.repo(),
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
    pub fn find_elastic(&self) -> Option<elastic::ElasticImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_elastic() {
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
    pub fn find_lss(&self) -> Option<lss::LssImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_lss() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_broker(&self) -> Option<broker::BrokerImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_broker() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_tribes(&self) -> Option<tribes::TribesImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_tribes() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_bot(&self) -> Option<bot::BotImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_bot() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_builtin(&self) -> Option<builtin::BuiltinImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_builtin() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_dufs(&self) -> Option<dufs::DufsImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_dufs() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_whisper(&self) -> Option<whisper::WhisperImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_whisper() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_llama(&self) -> Option<llama::LlamaImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_llama() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_mongo(&self) -> Option<mongo::MongoImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_mongo() {
                return Some(i);
            }
        }
        None
    }
    pub fn find_redis(&self) -> Option<redis::RedisImage> {
        for img in self.0.iter() {
            if let Ok(i) = img.as_redis() {
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
    pub fn as_elastic(&self) -> anyhow::Result<elastic::ElasticImage> {
        match self {
            Image::Elastic(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Elastic".to_string())),
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
    pub fn as_lss(&self) -> anyhow::Result<lss::LssImage> {
        match self {
            Image::Lss(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not LSS".to_string())),
        }
    }
    pub fn as_broker(&self) -> anyhow::Result<broker::BrokerImage> {
        match self {
            Image::Broker(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Broker".to_string())),
        }
    }
    pub fn as_tribes(&self) -> anyhow::Result<tribes::TribesImage> {
        match self {
            Image::Tribes(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Tribes".to_string())),
        }
    }
    pub fn as_bot(&self) -> anyhow::Result<bot::BotImage> {
        match self {
            Image::Bot(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Bot".to_string())),
        }
    }
    pub fn as_builtin(&self) -> anyhow::Result<builtin::BuiltinImage> {
        match self {
            Image::Builtin(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Builtin".to_string())),
        }
    }
    pub fn as_dufs(&self) -> anyhow::Result<dufs::DufsImage> {
        match self {
            Image::Dufs(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Dufs".to_string())),
        }
    }
    pub fn as_whisper(&self) -> anyhow::Result<whisper::WhisperImage> {
        match self {
            Image::Whisper(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Whisper".to_string())),
        }
    }
    pub fn as_mongo(&self) -> anyhow::Result<mongo::MongoImage> {
        match self {
            Image::Mongo(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Mongo".to_string())),
        }
    }
    pub fn as_llama(&self) -> anyhow::Result<llama::LlamaImage> {
        match self {
            Image::Llama(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Llama".to_string())),
        }
    }
    pub fn as_redis(&self) -> anyhow::Result<redis::RedisImage> {
        match self {
            Image::Redis(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not Redis".to_string())),
        }
    }
}

fn strarr(i: Vec<&str>) -> Vec<String> {
    i.iter().map(|s| s.to_string()).collect()
}
