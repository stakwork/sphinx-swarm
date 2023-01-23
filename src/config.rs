use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::conn::lnd::lndrpc::LndRPC;
use crate::conn::proxy::ProxyAPI;
use crate::conn::relay::RelayAPI;
use crate::images::{
    btc::BtcImage, cache::CacheImage, lnd::LndImage, proxy::ProxyImage, relay::RelayImage, Image,
};
use crate::utils;
use anyhow::Result;
use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(Default::default()));

pub struct State {
    pub stack: Stack,
    pub clients: Clients,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stack: Default::default(),
            clients: Default::default(),
        }
    }
}

pub struct Clients {
    pub bitcoind: HashMap<String, BitcoinRPC>,
    pub lnd: HashMap<String, LndRPC>,
    pub proxy: HashMap<String, ProxyAPI>,
    pub relay: HashMap<String, RelayAPI>,
}

impl Default for Clients {
    fn default() -> Self {
        Self {
            bitcoind: HashMap::new(),
            lnd: HashMap::new(),
            proxy: HashMap::new(),
            relay: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stack {
    // "bitcoin" or "regtest"
    pub network: String,
    pub nodes: Vec<Node>,
}

// optional node, could be external
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "place")]
pub enum Node {
    Internal(Image),
    External(ExternalNode),
}

impl Node {
    pub fn name(&self) -> String {
        match self {
            Node::Internal(n) => n.name(),
            Node::External(n) => n.name().clone(),
        }
    }
    pub fn as_internal(&self) -> Result<Image> {
        match self {
            Node::Internal(n) => Ok(n.clone()),
            Node::External(_n) => Err(anyhow::anyhow!("not an internal node".to_string())),
        }
    }
    pub fn as_external(&self) -> Result<ExternalNode> {
        match self {
            Node::Internal(_n) => Err(anyhow::anyhow!("not an external node".to_string())),
            Node::External(n) => Ok(n.clone()),
        }
    }
    pub fn is_ext_of_type(&self, typ: ExternalNodeType) -> bool {
        if let Ok(ext) = self.as_external() {
            if ext.kind == typ {
                return true;
            }
        }
        false
    }
}

impl Default for Stack {
    fn default() -> Self {
        let network = "regtest".to_string();
        // bitcoind
        let mut v = "v23.0";
        let bitcoind = BtcImage::new("bitcoind", v, &network, "sphinx");

        // lnd
        v = "v0.15.5-beta";
        let mut lnd = LndImage::new("lnd1", v, &network, "10009");
        lnd.http_port = Some("8881".to_string());
        lnd.links(vec!["bitcoind"]);

        // proxy
        v = "0.1.5";
        let mut proxy = ProxyImage::new("proxy1", v, &network, "11111", "5050");
        proxy.new_nodes(Some("0".to_string()));
        proxy.links(vec!["lnd1"]);

        // relay
        v = "v0.1.0";
        let node_env = "development";
        let mut relay = RelayImage::new("relay1", v, node_env, "3000");
        relay.links(vec!["proxy1", "lnd1", "tribes", "memes"]);

        // cache
        v = "0.1.14";
        let mut cache = CacheImage::new("cache1", v, "9000", true);
        cache.links(vec!["tribes", "lnd1"]);

        // internal nodes
        let internal_nodes = vec![
            Image::Btc(bitcoind),
            Image::Lnd(lnd),
            Image::Proxy(proxy),
            Image::Relay(relay),
            Image::Cache(cache),
        ];

        let mut nodes: Vec<Node> = internal_nodes
            .iter()
            .map(|n| Node::Internal(n.to_owned()))
            .collect();

        // external nodes
        nodes.push(Node::External(ExternalNode::new(
            "tribes",
            ExternalNodeType::Tribes,
            "tribes.sphinx.chat",
        )));

        nodes.push(Node::External(ExternalNode::new(
            "memes",
            ExternalNodeType::Meme,
            "meme.sphinx.chat",
        )));
        Stack { network, nodes }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ExternalNodeType {
    Bitcoind,
    Tribes,
    Meme,
    Postgres,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExternalNode {
    #[serde(rename = "type")]
    pub kind: ExternalNodeType,
    pub name: String,
    pub url: String,
}

impl ExternalNode {
    pub fn name(&self) -> String {
        self.name.to_string()
    }
}

impl ExternalNode {
    pub fn new(name: &str, kind: ExternalNodeType, url: &str) -> Self {
        Self {
            name: name.to_string(),
            kind,
            url: url.to_string(),
        }
    }
}

pub async fn load_config_file(project: &str) -> Stack {
    let path = format!("vol/{}/config.json", project);
    utils::load_json(&path, Default::default()).await
}
pub async fn get_config_file(project: &str) -> Stack {
    let path = format!("vol/{}/config.json", project);
    utils::get_json(&path).await
}
pub async fn put_config_file(project: &str, rs: &Stack) {
    let path = format!("vol/{}/config.json", project);
    utils::put_json(&path, rs).await
}
