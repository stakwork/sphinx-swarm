use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::conn::cln::ClnRPC;
use crate::conn::lnd::lndrpc::LndRPC;
use crate::conn::proxy::ProxyAPI;
use crate::conn::relay::RelayAPI;
use crate::images::boltwall::{BoltwallImage, ExternalLnd};
use crate::images::cln::{ClnImage, ClnPlugin};
use crate::images::jarvis::JarvisImage;
use crate::images::navfiber::NavFiberImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::{
    btc::BtcImage, cache::CacheImage, lnd::LndImage, lss::LssImage, proxy::ProxyImage,
    relay::RelayImage, Image,
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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct Stack {
    // "bitcoin" or "regtest"
    pub network: String,
    pub nodes: Vec<Node>,
    pub host: Option<String>, // root host for traefik (PRODUCTION)
    pub users: Vec<User>,
    pub jwt_key: String,
    pub ready: bool,
    pub ip: Option<String>,
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

fn only_second_brain(network: &str, host: Option<String>, lightning_provider: &str) -> Stack {
    Stack {
        network: network.to_string(),
        nodes: second_brain_imgs(host.clone(), lightning_provider)
            .iter()
            .map(|n| Node::Internal(n.to_owned()))
            .collect(),
        host,
        users: vec![Default::default()],
        jwt_key: secrets::random_word(16),
        ready: false,
        ip: env_no_empty("IP"),
    }
}

fn env_no_empty(varname: &str) -> Option<String> {
    match std::env::var(varname).ok() {
        Some(v) => match v.as_str() {
            "" => None,
            s => Some(s.to_string()),
        },
        None => None,
    }
}

fn external_lnd() -> Option<ExternalLnd> {
    if let Some(a) = env_no_empty("EXTERNAL_LND_ADDRESS") {
        if let Some(m) = env_no_empty("EXTERNAL_LND_MACAROON") {
            if let Some(c) = env_no_empty("EXTERNAL_LND_CERT") {
                return Some(ExternalLnd::new(&a, &m, &c));
            }
        }
    }
    None
}

fn second_brain_imgs(host: Option<String>, lightning_provider: &str) -> Vec<Image> {
    // neo4j
    let mut v = "4.4.9";
    let mut neo4j = Neo4jImage::new("neo4j", v);
    neo4j.host(host.clone());

    // jarvis
    v = "latest";
    let mut jarvis = JarvisImage::new("jarvis", v, "6000", false);
    jarvis.links(vec!["neo4j", "boltwall"]);

    // boltwall
    v = "latest";
    let mut bolt = BoltwallImage::new("boltwall", v, "8444");
    if let Some(ext) = external_lnd() {
        bolt.external_lnd(ext);
        bolt.links(vec!["jarvis"]);
    } else {
        bolt.links(vec!["jarvis", lightning_provider]);
    }
    bolt.host(host.clone());

    // navfiber
    v = "latest";
    let mut nav = NavFiberImage::new("navfiber", v, "8001");
    nav.links(vec!["jarvis"]);
    nav.host(host.clone());

    vec![
        Image::NavFiber(nav),
        Image::Neo4j(neo4j),
        Image::BoltWall(bolt),
        Image::Jarvis(jarvis),
    ]
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

        // host
        let mut host = std::env::var("HOST").ok();
        // must include a "."
        if let Some(h) = host.clone() {
            log::info!("HOST {:?}", h);
        }
        if !host.clone().unwrap_or(".".to_string()).contains(".") {
            host = None
        }

        let use_lnd = match std::env::var("USE_LND").ok() {
            Some(sbo) => sbo == "true",
            None => false,
        };
        // choose cln or lnd
        let is_cln = !use_lnd;
        let lightning_provider = if is_cln { "cln" } else { "lnd" };

        // choose only second brain
        let second_brain_only = match std::env::var("SECOND_BRAIN_ONLY").ok() {
            Some(sbo) => sbo == "true",
            None => false,
        };
        if second_brain_only {
            return only_second_brain(&network, host.clone(), lightning_provider);
        }

        let mut internal_nodes = vec![];
        let mut external_nodes = vec![];

        let mut external_btc = false;
        // CLN and external BTC
        if let Ok(ebtc) = std::env::var("CLN_MAINNET_BTC") {
            // check the BTC url is ok
            if let Ok(_) = url::Url::parse(&ebtc) {
                let btc = ExternalNode::new("bitcoind", ExternalNodeType::Btc, &ebtc);
                external_nodes.push(Node::External(btc));
                external_btc = true;
            }
        } else {
            if network == "bitcoin" {
                panic!("CLN_MAINNET_BTC required for mainnet");
            }
        }

        if !external_btc {
            let v = "v23.0";
            let mut bitcoind = BtcImage::new("bitcoind", v, &network);
            // connect to already running BTC node
            if let Ok(btc_pass) = std::env::var("BTC_PASS") {
                // only if its really there (not empty string)
                if btc_pass.len() > 0 {
                    bitcoind.set_user_password("sphinx", &btc_pass);
                }
            }
            // generate random pass if none exists
            if let None = bitcoind.pass {
                bitcoind.set_user_password("sphinx", &secrets::random_word(12));
            }
            internal_nodes.push(Image::Btc(bitcoind));
        }

        if is_cln {
            // lightning storage server
            let lss = LssImage::new("lss", "latest", "55551");
            internal_nodes.push(Image::Lss(lss));
            // cln with plugins
            let mut cln = ClnImage::new("cln", "latest", &network, "9735", "10009");
            cln.links(vec!["bitcoind", "lss"]);
            let skip_remote_signer = match std::env::var("NO_REMOTE_SIGNER").ok() {
                Some(nsb) => nsb == "true",
                None => false,
            };
            let plugins = if skip_remote_signer {
                vec![ClnPlugin::HtlcInterceptor]
            } else {
                vec![ClnPlugin::HsmdBroker, ClnPlugin::HtlcInterceptor]
            };
            cln.plugins(plugins);
            cln.host(host.clone());
            internal_nodes.push(Image::Cln(cln));
        } else {
            // lnd
            let v = "v0.16.2-beta";
            let mut lnd = LndImage::new("lnd", v, &network, "10009", "9735");
            lnd.http_port = Some("8881".to_string());
            lnd.links(vec!["bitcoind"]);
            lnd.host(host.clone());

            let mut lnd2 = LndImage::new("lnd2", v, &network, "10010", "9736");
            lnd2.http_port = Some("8882".to_string());
            lnd2.links(vec!["bitcoind"]);
            lnd2.host(host.clone());

            internal_nodes.push(Image::Lnd(lnd));
            internal_nodes.push(Image::Lnd(lnd2));
        }

        // proxy
        let mut v = "latest";
        let mut proxy = ProxyImage::new("proxy", v, &network, "11111", "5050");
        proxy.new_nodes(Some("0".to_string()));
        proxy.links(vec![lightning_provider]);

        // relay
        v = "latest";
        let node_env = match host {
            Some(_) => "production",
            None => "development",
        };
        let mut relay = RelayImage::new("relay", v, node_env, "3000");
        relay.dont_ping_hub();
        relay.links(vec![
            "proxy",
            lightning_provider,
            "tribes",
            "memes",
            "boltwall",
            "cache",
        ]);
        relay.host(host.clone());

        // cache
        v = "latest";
        let mut cache = CacheImage::new("cache", v, "9000", true);
        cache.links(vec!["tribes"]);

        // other_internal_nodes
        let other_internal_nodes = vec![
            Image::Proxy(proxy),
            Image::Relay(relay),
            Image::Cache(cache),
        ];
        internal_nodes.extend(other_internal_nodes);

        // NO_SECOND_BRAIN=true will skip these nodes
        let skip_second_brain = match std::env::var("NO_SECOND_BRAIN").ok() {
            Some(nsb) => nsb == "true",
            None => false,
        };
        if !skip_second_brain {
            let second_brain_nodes = second_brain_imgs(host.clone(), lightning_provider);
            internal_nodes.extend(second_brain_nodes);
        }

        let mut nodes: Vec<Node> = internal_nodes
            .iter()
            .map(|n| Node::Internal(n.to_owned()))
            .collect();

        // external nodes
        external_nodes.push(Node::External(ExternalNode::new(
            "tribes",
            ExternalNodeType::Tribes,
            "tribes.sphinx.chat",
        )));
        external_nodes.push(Node::External(ExternalNode::new(
            "memes",
            ExternalNodeType::Meme,
            "meme.sphinx.chat",
        )));

        // final nodes array
        nodes.extend(external_nodes);

        let mut ip: Option<String> = None;

        Stack {
            network,
            nodes,
            host,
            users: vec![Default::default()],
            jwt_key: secrets::random_word(16),
            ready: false,
            ip: env_no_empty("IP"),
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
                Image::Lss(l) => Node::Internal(Image::Lss(l)),
            },
        });
        Stack {
            network: self.network.clone(),
            nodes: nodes.collect(),
            host: self.host.clone(),
            users: vec![],
            jwt_key: "".to_string(),
            ready: self.ready,
            ip: self.ip.clone(),
        }
    }
}
