use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::conn::lnd::lndrpc::LndRPC;
use crate::images::{BtcImage, Image, LndImage, ProxyImage, RelayImage};
use crate::utils;
use anyhow::Result;
use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

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
}
impl Default for Clients {
    fn default() -> Self {
        Self {
            bitcoind: HashMap::new(),
            lnd: HashMap::new(),
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
            Node::External(n) => n.url.clone(),
        }
    }
    pub fn as_internal(&self) -> Result<Image> {
        match self {
            Node::Internal(n) => Ok(n.clone()),
            Node::External(n) => Err(anyhow::anyhow!("not an internal node".to_string())),
        }
    }
    pub fn as_btc(&self) -> Result<BtcImage> {
        match self.as_internal()? {
            Image::Btc(i) => Ok(i),
            _ => Err(anyhow::anyhow!("not a BTC image".to_string())),
        }
    }
    pub fn as_lnd(&self) -> Result<LndImage> {
        match self.as_internal()? {
            Image::Lnd(i) => Ok(i),
            _ => Err(anyhow::anyhow!("not a LND image".to_string())),
        }
    }
}

impl Default for Stack {
    fn default() -> Self {
        let network = "regtest".to_string();
        // bitcoind
        let mut v = "23.0";
        let bitcoind = BtcImage::new("bitcoind", v, &network, "sphinx");
        // lnd
        v = "v0.14.3-beta.rc1";
        let mut lnd = LndImage::new("lnd1", v, &network, "10009");
        lnd.http_port = Some("8881".to_string());
        lnd.links(vec!["bitcoind"]);


        // proxy
        v = "0.1.2";
        let mut proxy = ProxyImage::new("proxy1", v, &network, "11111", "5050");
        proxy.new_nodes(Some("0".to_string()));
        proxy.links(vec!["lnd1"]);
        // relay
        v = "v2.2.12";
        let mut relay = RelayImage::new("relay1", v, "3000");
        relay.links(vec!["proxy1", "lnd1"]);
        // internal nodes
        let internal_nodes = vec![
            Image::Btc(bitcoind),
            Image::Lnd(lnd),
            Image::Proxy(proxy),
            Image::Relay(relay),
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

// #[serde(skip_serializing_if = "Option::is_none")]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct RelayConfig {
    pub node_ip: String,
    pub lnd_ip: String,
    pub lnd_port: String,
    pub public_url: String,
    pub tls_location: String,
    pub macaroon_location: String,
    pub node_http_port: String,
    pub tribes_mqtt_port: String,
    pub db_dialect: String,
    pub db_storage: String,
    pub tribes_insecure: Option<String>,
    pub node_http_protocol: Option<String>,
    pub transport_private_key_location: Option<String>,
    pub transport_public_key_location: Option<String>,
    pub proxy_macaroons_dir: Option<String>,
    pub proxy_tls_location: Option<String>,
    pub proxy_lnd_ip: Option<String>,
    pub proxy_lnd_port: Option<String>,
    pub proxy_admin_token: Option<String>,
    pub proxy_admin_url: Option<String>,
    pub proxy_new_nodes: Option<String>,
    pub proxy_initial_sats: Option<String>,
}

impl RelayConfig {
    pub fn new(_name: &str, port: &str) -> Self {
        Self {
            node_http_port: port.to_string(),
            public_url: format!("127.0.0.1:{}", port).to_string(),
            ..Default::default()
        }
    }
    pub fn lnd(&mut self, lnd: &LndImage) {
        self.lnd_ip = format!("{}.sphinx", lnd.name);
        self.lnd_port = lnd.port.to_string();
        self.tls_location = "/lnd/tls.cert".to_string();
        self.macaroon_location = "/lnd/data/chain/bitcoin/regtest/admin.macaroon".to_string();
    }
    pub fn proxy(&mut self, proxy: &ProxyImage) {
        self.proxy_lnd_ip = Some(format!("{}.sphinx", proxy.name));
        self.proxy_lnd_port = Some(proxy.port.clone());
        self.proxy_admin_token = proxy.admin_token.clone();
        self.proxy_macaroons_dir = Some("/proxy/macaroons".to_string());
        self.proxy_tls_location = Some("/proxy/tls.cert".to_string());
        self.proxy_admin_url = Some(format!("{}.sphinx:{}", proxy.name, proxy.admin_port));
        self.proxy_new_nodes = proxy.new_nodes.clone();
    }
}

type JsonMap = HashMap<String, String>;

pub fn relay_env_config(c: &RelayConfig) -> Vec<String> {
    let blah = serde_json::to_value(&c).unwrap();
    let conf: JsonMap = serde_json::from_value(blah).unwrap();
    let mut ret = Vec::new();
    for (k, v) in conf.iter() {
        ret.push(format!("{}={}", k, v));
    }
    ret
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            node_ip: "127.0.0.1".to_string(),
            lnd_ip: "lnd-dev.sphinx".to_string(),
            lnd_port: "10009".to_string(),
            public_url: "127.0.0.0:3000".to_string(),
            tls_location: "/relay/.lnd/tls.cert".to_string(),
            macaroon_location: "/relay/.lnd/data/chain/bitcoin/regtest/admin.macaroon".to_string(),
            node_http_port: "3000".to_string(),
            tribes_mqtt_port: "1883".to_string(),
            db_dialect: "sqlite".to_string(),
            db_storage: "/relay/data/sphinx.db".to_string(),
            node_http_protocol: None,
            tribes_insecure: None,
            transport_private_key_location: None,
            transport_public_key_location: None,
            proxy_macaroons_dir: None,
            proxy_tls_location: None,
            proxy_lnd_ip: None,
            proxy_lnd_port: None,
            proxy_admin_token: None,
            proxy_admin_url: None,
            proxy_new_nodes: None,
            proxy_initial_sats: None,
        }
    }
}

// using env instead of file
pub fn _relay_config(project: &str, name: &str) -> RelayConfig {
    let path = format!("vol/{}/{}.json", project, name);
    match fs::read(path.clone()) {
        Ok(data) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => Default::default(),
        },
        Err(_e) => {
            let st = serde_json::to_string_pretty::<RelayConfig>(&Default::default())
                .expect("failed to make json string");
            let mut file = File::create(path).expect("create failed");
            file.write_all(st.as_bytes()).expect("write failed");
            Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_relay_config() {
        let mut c = RelayConfig::new("relay", "3000");
        c.lnd(&LndImage::new(
            "lnd",
            "v0.14.3-beta.rc1",
            "regtest",
            "10009",
        ));
        relay_env_config(&c);
        assert!(true == true)
    }
}

// pub async fn get_conf() -> &'static Config {
//     let conf = CONFIG.lock().await;
//     &conf
// }
