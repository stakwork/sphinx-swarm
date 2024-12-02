use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::conn::cln::hsmd::HsmdClient;
use crate::conn::cln::ClnRPC;
use crate::conn::lnd::lndrpc::LndRPC;
use crate::conn::proxy::ProxyAPI;
use crate::conn::relay::RelayAPI;
use crate::images::Image;
use crate::utils;
use anyhow::Result;
use once_cell::sync::Lazy;
use rocket::tokio;
use rocket::tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;

pub static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(Default::default()));

pub static GLOBAL_MEM_LIMIT: AtomicU64 = AtomicU64::new(0);

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
    pub hsmd: HashMap<String, HsmdClient>,
}

impl Default for Clients {
    fn default() -> Self {
        Self {
            bitcoind: HashMap::new(),
            lnd: HashMap::new(),
            cln: HashMap::new(),
            proxy: HashMap::new(),
            relay: HashMap::new(),
            hsmd: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Stack {
    // "bitcoin" or "regtest"
    pub network: String,
    pub nodes: Vec<Node>,
    pub host: Option<String>, // root host for traefik (PRODUCTION)
    pub users: Vec<User>,
    pub jwt_key: String,
    pub ready: bool,
    pub ip: Option<String>,
    pub auto_update: Option<Vec<String>>,
    pub auto_restart: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_2b_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_mem_limit: Option<u64>,
    pub backup_services: Option<Vec<String>>,
    pub lightning_peers: Option<Vec<LightningPeer>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum Role {
    Admin,
    SubAdmin,
    Super,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct User {
    pub id: u32,
    pub username: String,
    pub pass_hash: String,
    pub pubkey: Option<String>,
    pub role: Role,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct LightningPeer {
    pub alias: String,
    pub pubkey: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SendSwarmDetailsBody {
    pub username: String,
    pub password: String,
    pub host: String,
    pub default_host: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SendSwarmDetailsResponse {
    pub message: String,
}

// optional node, could be external
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
            pubkey: None,
            role: Role::Admin,
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
                Image::Elastic(n) => Node::Internal(Image::Elastic(n)),
                Image::NavFiber(nf) => Node::Internal(Image::NavFiber(nf)),
                Image::Jarvis(j) => Node::Internal(Image::Jarvis(j)),
                Image::BoltWall(mut b) => {
                    b.session_secret = "".to_string();
                    Node::Internal(Image::BoltWall(b))
                }
                Image::Lss(l) => Node::Internal(Image::Lss(l)),
                Image::Broker(mut b) => {
                    b.seed = "".to_string();
                    Node::Internal(Image::Broker(b))
                }
                Image::Mixer(m) => Node::Internal(Image::Mixer(m)),
                Image::Tribes(t) => Node::Internal(Image::Tribes(t)),
                Image::Config(c) => Node::Internal(Image::Config(c)),
                Image::Bot(mut b) => {
                    b.seed = "".to_string();
                    b.admin_token = "".to_string();
                    Node::Internal(Image::Bot(b))
                }
                Image::Builtin(b) => Node::Internal(Image::Builtin(b)),
                Image::Dufs(d) => Node::Internal(Image::Dufs(d)),
                Image::Tome(mut m) => {
                    m.jwt_secret = "".to_string();
                    Node::Internal(Image::Tome(m))
                }
                Image::Rqbit(r) => Node::Internal(Image::Rqbit(r)),
                Image::Llama(m) => Node::Internal(Image::Llama(m)),
                Image::Whisper(w) => Node::Internal(Image::Whisper(w)),
                Image::Whisker(mut w) => {
                    w.livekit_api_key = "".to_string();
                    w.livekit_api_secret = "".to_string();
                    Node::Internal(Image::Whisker(w))
                }
                Image::Runner(r) => Node::Internal(Image::Runner(r)),
                Image::Mongo(m) => Node::Internal(Image::Mongo(m)),
                Image::Jamie(c) => Node::Internal(Image::Jamie(c)),
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
            auto_update: self.auto_update.clone(),
            auto_restart: self.auto_restart.clone(),
            custom_2b_domain: self.custom_2b_domain.clone(),
            global_mem_limit: self.global_mem_limit,
            backup_services: self.backup_services.clone(),
            lightning_peers: self.lightning_peers.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
struct GbmRes {
    global_mem_limit: u64,
}
pub fn set_global_mem_limit(gbm: u64) -> Result<String> {
    log::info!("Set Global Memory Limit ===> {:?}", gbm);
    use std::sync::atomic::Ordering;
    GLOBAL_MEM_LIMIT.store(gbm, Ordering::Relaxed);
    Ok(serde_json::to_string(&GbmRes {
        global_mem_limit: gbm,
    })?)
}
