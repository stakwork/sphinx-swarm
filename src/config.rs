use crate::images::{LndNode, ProxyNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

#[derive(Serialize, Deserialize)]
pub struct Config {
    // "bitcoin" or "regtest"
    pub network: String,
    // external bitcoind provider
    pub bitcoind: Option<String>,
    // external postgres provider
    pub postgres: Option<String>,
    // external tribes provider
    pub tribes: Option<String>,
    // external meme provider
    pub meme: Option<String>,
    // extra lnd+relay instances
    pub lnds: Vec<ImageConfig>,
    // extra cln+relay instances
    pub clns: Vec<ImageConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct ImageConfig {
    pub name: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            network: "bitcoin".to_string(),
            bitcoind: None,
            postgres: None,
            tribes: None,
            meme: None,
            lnds: vec![],
            clns: vec![],
        }
    }
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
}

impl RelayConfig {
    pub fn new(name: &str, port: &str) -> Self {
        Self {
            node_http_port: port.to_string(),
            public_url: format!("127.0.0.1:{}", port).to_string(),
            ..Default::default()
        }
    }
    pub fn lnd(&mut self, lnd: &LndNode) {
        self.lnd_ip = format!("{}.sphinx", lnd.name);
        self.lnd_port = lnd.port.to_string();
        self.tls_location = format!("{}/tls.cert", lnd.dir).to_string();
        self.macaroon_location =
            format!("{}/data/chain/bitcoin/regtest/admin.macaroon", lnd.dir).to_string();
    }
    pub fn proxy(&mut self, proxy: &ProxyNode, root_dir: &str) {
        self.proxy_lnd_ip = Some(format!("{}.sphinx", proxy.name));
        self.proxy_lnd_port = Some(proxy.port.clone());
        self.proxy_admin_token = Some(proxy.admin_token.clone());
        self.proxy_macaroons_dir = Some(format!("{}/macaroons", root_dir));
        self.proxy_tls_location = Some(format!("{}/cert/tls.cert", root_dir));
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
            db_storage: "/relay/sphinx.db".to_string(),
            node_http_protocol: None,
            tribes_insecure: None,
            transport_private_key_location: None,
            transport_public_key_location: None,
            proxy_macaroons_dir: None,
            proxy_tls_location: None,
            proxy_lnd_ip: None,
            proxy_lnd_port: None,
            proxy_admin_token: None,
        }
    }
}

// using env instead of file
pub fn _relay_config(project: &str, name: &str) -> Config {
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
        c.lnd(&LndNode::new("lnd", "regtest", "10009", "/.lnd/"));
        relay_env_config(&c);
        assert!(true == true)
    }
}
