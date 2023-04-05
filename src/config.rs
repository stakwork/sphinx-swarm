use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::conn::cln::ClnRPC;
use crate::conn::lnd::lndrpc::LndRPC;
use crate::conn::proxy::ProxyAPI;
use crate::conn::relay::RelayAPI;
use crate::images::boltwall::BoltwallImage;
use crate::images::jarvis::JarvisImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::{
    btc::BtcImage, cache::CacheImage, lnd::LndImage, proxy::ProxyImage, relay::RelayImage, Image,
};
use crate::secrets;
use crate::utils;
use anyhow::Result;
use once_cell::sync::Lazy;
use rocket::tokio;
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
    pub cln: HashMap<String, ClnRPC>,
    pub proxy: HashMap<String, ProxyAPI>,
    pub relay: HashMap<String, RelayAPI>,
}

impl Default for Clients {
    fn default() -> Self {
        Self {
            bitcoind: HashMap::new(),
            lnd: HashMap::new(),
            cln: HashMap::new(),
            proxy: HashMap::new(),
            relay: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Stack {
    // "bitcoin" or "regtest"
    pub network: String,
    pub nodes: Vec<Node>,
    pub host: Option<String>, // root host for traefik (PRODUCTION)
    pub users: Vec<User>,
    pub jwt_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub pass_hash: String,
}

// optional node, could be external
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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
    pub fn set_version(&mut self, version: &str) -> Result<()> {
        match self {
            Node::Internal(img) => {
                img.set_version(version);
                Ok(())
            }
            Node::External(_n) => Err(anyhow::anyhow!("not an internal node".to_string())),
        }
    }
}

impl Default for User {
    fn default() -> Self {
        let username = "admin";
        let default_password = "password";
        let pass_hash =
            bcrypt::hash(default_password, bcrypt::DEFAULT_COST).expect("failed to bcrypt");
        Self {
            id: 1,
            username: username.to_string(),
            pass_hash,
        }
    }
}

// NETWORK = "bitcoin", "regtest"
// HOST = hostname for this server (swarmx.sphinx.chat)
// BTC_PASS = already created BTC password
// ONLY_NODE = start up just one node
impl Default for Stack {
    fn default() -> Self {
        // network
        let mut network = "regtest".to_string();
        if let Ok(env_net) = std::env::var("NETWORK") {
            if env_net == "bitcoin" || env_net == "regtest" {
                network = env_net;
            }
        }

        let mut host = std::env::var("HOST").ok();
        // must include a "."
        if let Some(h) = host.clone() {
            log::info!("HOST {:?}", h);
        }
        if !host.clone().unwrap_or(".".to_string()).contains(".") {
            host = None
        }

        // bitcoind
        let mut v = "v23.0";
        let mut bitcoind = BtcImage::new("bitcoind", v, &network);
        // connect to already running BTC node
        if let Ok(btc_pass) = std::env::var("BTC_PASS") {
            // only if its really there (not empty string)
            if btc_pass.len() > 0 {
                bitcoind.set_user_password("sphinx", &btc_pass);
            }
        } else {
            bitcoind.set_user_password("sphinx", &secrets::random_word(12));
        }

        // lnd
        v = "v0.15.5-beta";
        let mut lnd = LndImage::new("lnd", v, &network, "10009", "9735");
        lnd.http_port = Some("8881".to_string());
        lnd.links(vec!["bitcoind"]);
        lnd.host(host.clone());

        // lnd2
        // let mut lnd2 = LndImage::new("lnd2", v, &network, "10010", "9736");
        // lnd2.http_port = Some("8882".to_string());
        // lnd2.links(vec!["bitcoind", "lnd"]);

        // proxy
        v = "0.1.18";
        let mut proxy = ProxyImage::new("proxy", v, &network, "11111", "5050");
        proxy.new_nodes(Some("0".to_string()));
        proxy.links(vec!["lnd"]);

        // relay
        v = "v0.1.17";
        let node_env = match host {
            Some(_) => "production",
            None => "development",
        };
        let mut relay = RelayImage::new("relay", v, node_env, "3000");
        relay.links(vec!["proxy", "lnd", "tribes", "memes", "boltwall", "cache"]);
        relay.host(host.clone());

        // cache
        v = "0.1.17";
        let mut cache = CacheImage::new("cache", v, "9000", true);
        cache.links(vec!["tribes"]);

        // neo4j
        v = "4.4.9";
        let neo4j = Neo4jImage::new("neo4j", v);

        // jarvis
        v = "0.1";
        let mut jarvis = JarvisImage::new("jarvis", v, "6000");
        jarvis.links(vec!["neo4j"]);

        // boltwall
        v = "latest";
        let mut bolt = BoltwallImage::new("boltwall", v, "8444");
        bolt.links(vec!["jarvis"]);
        bolt.host(host.clone());

        // navfiber
        v = "latest";
        let mut nav = NavFiberImage::new("navfiber", v, "8001");
        nav.links(vec!["jarvis"]);
        nav.host(host.clone());

        // internal nodes
        let mut internal_nodes = vec![
            Image::Btc(bitcoind),
            Image::Lnd(lnd),
            Image::Proxy(proxy),
            Image::Relay(relay),
            Image::Cache(cache),
        ];

        // NO_SECOND_BRAIN=true will skip these nodes
        let skip_second_brain = match std::env::var("NO_SECOND_BRAIN").ok() {
            Some(nsb) => nsb == "true",
            None => false,
        };
        if !skip_second_brain {
            let second_brain_nodes = vec![
                Image::NavFiber(nav),
                Image::Neo4j(neo4j),
                Image::BoltWall(bolt),
                Image::Jarvis(jarvis),
            ];
            internal_nodes.extend(second_brain_nodes);
        }

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
        Stack {
            network,
            nodes,
            host,
            users: vec![Default::default()],
            jwt_key: secrets::random_word(16),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ExternalNodeType {
    Btc,
    Tribes,
    Meme,
    Postgres,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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

async fn file_exists(file: &str) -> bool {
    let path = std::path::Path::new(&file);
    tokio::fs::metadata(path).await.is_ok()
}

const YAML: bool = true;

pub async fn load_config_file(project: &str) -> Result<Stack> {
    let path = format!("vol/{}/config.json", project);
    if !YAML {
        return Ok(utils::load_json(&path, Default::default()).await);
    }
    let yaml_path = format!("vol/{}/config.yaml", project);
    if file_exists(&path).await {
        // migrate to yaml
        let stack: Stack = utils::load_json(&path, Default::default()).await;
        // create the yaml version
        utils::put_yaml(&yaml_path, &stack).await;
        // delete the json version
        let _ = tokio::fs::remove_file(path).await;
        Ok(stack)
    } else {
        let s = utils::load_yaml(&yaml_path, Default::default()).await?;
        println!("STACK! {:?}", s);
        Ok(s)
    }
}

pub async fn put_config_file(project: &str, rs: &Stack) {
    let ext = if YAML { "yaml" } else { "json" };
    let path = format!("vol/{}/config.{}", project, ext);
    if YAML {
        utils::put_yaml(&path, rs).await
    } else {
        utils::put_json(&path, rs).await
    }
}

impl Stack {
    // remove sensitive data from Stack when sending over wire
    pub fn remove_tokens(&self) -> Stack {
        let nodes = self.nodes.iter().map(|n| match n {
            Node::External(e) => Node::External(e.clone()),
            Node::Internal(i) => match i.clone() {
                Image::Btc(mut b) => {
                    b.user = None;
                    b.pass = None;
                    Node::Internal(Image::Btc(b))
                }
                Image::Lnd(mut l) => {
                    l.unlock_password = "".to_string();
                    Node::Internal(Image::Lnd(l))
                }
                Image::Proxy(mut p) => {
                    p.store_key = None;
                    p.admin_token = None;
                    Node::Internal(Image::Proxy(p))
                }
                Image::Cln(c) => Node::Internal(Image::Cln(c)),
                Image::Relay(r) => Node::Internal(Image::Relay(r)),
                Image::Cache(c) => Node::Internal(Image::Cache(c)),
                Image::Neo4j(n) => Node::Internal(Image::Neo4j(n)),
                Image::NavFiber(nf) => Node::Internal(Image::NavFiber(nf)),
                Image::Jarvis(j) => Node::Internal(Image::Jarvis(j)),
                Image::BoltWall(mut b) => {
                    b.session_secret = "".to_string();
                    Node::Internal(Image::BoltWall(b))
                }
            },
        });
        Stack {
            network: self.network.clone(),
            nodes: nodes.collect(),
            host: self.host.clone(),
            users: vec![],
            jwt_key: "".to_string(),
        }
    }
}
