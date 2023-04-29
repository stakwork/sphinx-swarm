use super::traefik::traefik_labels;
use super::*;
use crate::config::{Clients, Node};
use crate::conn::relay::setup::relay_client;
use crate::images::lnd::to_lnd_network;
use crate::utils::{domain, exposed_ports, host_config, volume_string};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

// in relay:
// docker build --no-cache -f Dockerfile.swarm .

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct RelayImage {
    pub name: String,
    pub version: String,
    pub node_env: String,
    pub port: String,
    pub links: Links,
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dont_ping_hub: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<String>,
}
impl RelayImage {
    pub fn new(name: &str, version: &str, node_env: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            node_env: node_env.to_string(),
            port: port.to_string(),
            links: vec![],
            host: None,
            dont_ping_hub: None,
            logging: None,
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
    pub fn dont_ping_hub(&mut self) {
        self.dont_ping_hub = Some(true);
    }
    pub async fn connect_client(&self, proj: &str, clients: &mut Clients) -> Result<()> {
        match relay_client(proj, self).await {
            Ok(client) => {
                clients.relay.insert(self.name.clone(), client);
            }
            Err(e) => log::warn!("relay_client error: {:?}", e),
        };
        Ok(())
    }
}

#[async_trait]
impl DockerConfig for RelayImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let lnd = li.find_lnd();
        let cln = li.find_cln();
        let proxy = li.find_proxy();
        Ok(relay(&self, lnd, cln, proxy))
    }
}

impl DockerHubImage for RelayImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-relay-swarm".to_string(),
        }
    }
}

fn relay(
    relay: &RelayImage,
    lnd_opt: Option<lnd::LndImage>,
    cln_opt: Option<cln::ClnImage>,
    proxy: Option<proxy::ProxyImage>,
) -> Config<String> {
    // let img = "sphinx-relay";
    // let version = "latest";
    let repo = relay.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let version = relay.version.clone();
    let root_vol = "/relay/data";
    let mut conf = RelayConfig::new(&relay.name, &relay.port);
    if let Some(b) = relay.dont_ping_hub {
        if b {
            conf.dont_ping_hub();
        }
    }
    if let Some(lg) = &relay.logging {
        conf.logging(&lg);
    }
    let mut extra_vols = vec![];
    if let Some(lnd) = lnd_opt {
        conf.lnd(&lnd, "lnd");
        // add the LND volumes
        let lnd_vol = volume_string(&lnd.name, "/lnd");
        extra_vols.push(lnd_vol);
    }
    if let Some(cln) = cln_opt {
        conf.cln(&cln, "cln");
        // add the CLN volume
        let cln_vol = volume_string(&cln.name, "/cln");
        extra_vols.push(cln_vol);
    }
    // add the optional Proxy stuff
    if let Some(p) = proxy {
        conf.proxy(&p);
        let proxy_vol = volume_string(&p.name, "/proxy");
        extra_vols.push(proxy_vol);
    }
    // relay config from env
    let mut relay_conf = relay_env_config(&conf);
    relay_conf.push(format!("NODE_ENV={}", &relay.node_env));
    let mut c = Config {
        image: Some(format!("{}:{}", img, version)),
        hostname: Some(domain(&relay.name)),
        exposed_ports: exposed_ports(vec![relay.port.clone()]),
        host_config: host_config(
            &relay.name,
            vec![relay.port.clone()],
            root_vol,
            Some(extra_vols),
        ),
        env: Some(relay_conf),
        ..Default::default()
    };
    if let Some(host) = relay.host.clone() {
        c.labels = Some(traefik_labels(&relay.name, &host, &relay.port, true));
    }
    c
}

// #[serde(skip_serializing_if = "Option::is_none")]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct RelayConfig {
    pub lightning_provider: String,
    pub logging: Option<String>,
    pub node_ip: String,
    pub lnd_ip: String,
    pub lnd_port: String,
    pub public_url: String,
    pub node_http_port: String,
    pub db_dialect: String,
    pub db_storage: String,
    pub tribes_mqtt_port: String,
    pub tribes_host: String,
    pub people_host: String,
    pub tls_location: Option<String>,      // lnd
    pub macaroon_location: Option<String>, // lnd
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
    pub proxy_hd_keys: Option<String>,
    pub cln_ca_cert: Option<String>,
    pub cln_device_key: Option<String>,
    pub cln_device_cert: Option<String>,
    pub dont_ping_hub: Option<String>,
}

impl RelayConfig {
    pub fn new(_name: &str, port: &str) -> Self {
        Self {
            node_http_port: port.to_string(),
            public_url: format!("127.0.0.1:{}", port).to_string(),
            ..Default::default()
        }
    }
    pub fn dont_ping_hub(&mut self) {
        self.dont_ping_hub = Some("true".to_string());
    }
    pub fn logging(&mut self, logging: &str) {
        self.logging = Some(logging.to_string());
    }
    pub fn lnd(&mut self, lnd: &lnd::LndImage, root_vol_dir: &str) {
        self.lightning_provider = "LND".to_string();
        self.lnd_ip = domain(&lnd.name);
        self.lnd_port = lnd.rpc_port.to_string();
        self.tls_location = Some(format!("/{}/tls.cert", root_vol_dir));
        let netwk = to_lnd_network(lnd.network.as_str());
        self.macaroon_location = Some(format!(
            "/{}/data/chain/bitcoin/{}/admin.macaroon",
            root_vol_dir, netwk
        ));
    }
    pub fn cln(&mut self, cln: &cln::ClnImage, root_vol_dir: &str) {
        self.lightning_provider = "CLN".to_string();
        self.lnd_ip = domain(&cln.name);
        self.lnd_port = cln.grpc_port.to_string();
        let creds = cln.credentials_paths(root_vol_dir);
        self.cln_ca_cert = Some(creds.ca_cert);
        self.cln_device_cert = Some(creds.client_cert);
        self.cln_device_key = Some(creds.client_key);
    }
    pub fn proxy(&mut self, proxy: &proxy::ProxyImage) {
        self.proxy_lnd_ip = Some(domain(&proxy.name));
        self.proxy_lnd_port = Some(proxy.port.clone());
        self.proxy_admin_token = proxy.admin_token.clone();
        self.proxy_macaroons_dir = Some("/proxy/macaroons".to_string());
        self.proxy_tls_location = Some("/proxy/tls.cert".to_string());
        self.proxy_admin_url = Some(format!(
            "http://{}:{}",
            domain(&proxy.name),
            proxy.admin_port
        ));
        self.proxy_new_nodes = proxy.new_nodes.clone();
        self.proxy_hd_keys = Some("true".to_string());
    }
}

type JsonMap = std::collections::HashMap<String, String>;

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
        // let logging = "LIGHTNING,TRIBES,MEME,NOTIFICATION,EXPRESS,NETWORK,DB,PROXY,LSAT,BOTS";
        Self {
            lightning_provider: "LND".to_string(),
            logging: None,
            node_ip: "127.0.0.1".to_string(),
            lnd_ip: domain("lnd"),
            lnd_port: "10009".to_string(),
            public_url: "127.0.0.0:3000".to_string(),
            node_http_port: "3000".to_string(),
            db_dialect: "sqlite".to_string(),
            db_storage: "/relay/data/sphinx.db".to_string(),
            node_http_protocol: None,
            tribes_insecure: None,
            tribes_mqtt_port: "8883".to_string(),
            tribes_host: "tribes.sphinx.chat".to_string(),
            people_host: "people.sphinx.chat".to_string(),
            tls_location: None,
            macaroon_location: None,
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
            proxy_hd_keys: None,
            cln_ca_cert: None,
            cln_device_cert: None,
            cln_device_key: None,
            dont_ping_hub: None,
        }
    }
}

// using env instead of file
pub fn _relay_config(project: &str, name: &str) -> RelayConfig {
    use std::fs;
    use std::io::Write;
    // not using vol/ anymore...
    let path = format!("vol/{}/{}.json", project, name);
    match fs::read(path.clone()) {
        Ok(data) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => Default::default(),
        },
        Err(_e) => {
            let st = serde_json::to_string_pretty::<RelayConfig>(&Default::default())
                .expect("failed to make json string");
            let mut file = fs::File::create(path).expect("create failed");
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
        c.lnd(
            &lnd::LndImage::new("lnd", "v0.14.3-beta.rc1", "regtest", "10009", "9735"),
            "lnd",
        );
        relay_env_config(&c);
        assert!(true == true)
    }
}

// pub async fn get_conf() -> &'static Config {
//     let conf = CONFIG.lock().await;
//     &conf
// }
