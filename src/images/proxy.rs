use super::*;
use crate::images::lnd::to_lnd_network;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config, volume_string};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub port: String,
    pub admin_port: String,
    pub admin_token: Option<String>,
    pub store_key: Option<String>,
    pub new_nodes: Option<String>, // for relay
    pub links: Links,
}

impl ProxyImage {
    pub fn new(name: &str, version: &str, network: &str, port: &str, admin_port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            port: port.to_string(),
            admin_port: admin_port.to_string(),
            admin_token: Some(secrets::random_word(12)),
            store_key: Some(secrets::hex_secret()),
            new_nodes: None,
            links: vec![],
        }
    }
    pub fn new_nodes(&mut self, new_nodes: Option<String>) {
        self.new_nodes = new_nodes;
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}
impl DockerHubImage for ProxyImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-proxy".to_string(),
        }
    }
}

pub fn proxy(proxy: &ProxyImage, lnd: &lnd::LndImage) -> Config<String> {
    let repo = proxy.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let version = proxy.version.clone();
    // let img = "sphinx-proxy";
    // let version = "latest";
    let netwk = to_lnd_network(proxy.network.as_str());
    let macpath = format!(
        "--macaroon-location=/lnd/data/chain/bitcoin/{}/admin.macaroon",
        &netwk,
    );
    let root_vol = "/app/proxy";
    let lnd_vol = volume_string(&lnd.name, "/lnd");
    let extra_vols = vec![lnd_vol];
    let ports = vec![proxy.port.clone(), proxy.admin_port.clone()];
    let mut cmd = vec![
        "/app/sphinx-proxy".to_string(),
        macpath.to_string(),
        "--rpclisten=0.0.0.0:11111".to_string(),
        "--store-dir=/app/proxy/badger".to_string(),
        "--bitcoin.active".to_string(),
        "--bitcoin.basefee=0".to_string(),
        format!("--bitcoin.{}", &netwk),
        format!("--rpclisten=0.0.0.0:{}", &proxy.port),
        format!("--admin-port={}", &proxy.admin_port),
        format!("--lnd-ip={}.sphinx", &lnd.name),
        format!("--lnd-port={}", &lnd.rpc_port),
        format!("--tlsextradomain={}.sphinx", proxy.name),
        "--tlscertpath=/app/proxy/tls.cert".to_string(),
        "--tlskeypath=/app/proxy/tls.key".to_string(),
        "--tls-location=/lnd/tls.cert".to_string(),
        "--unlock-pwd=hi123456".to_string(),
        "--server-macaroons-dir=/app/proxy/macaroons".to_string(),
        "--channels-start=1".to_string(),
        "--initial-msat=0".to_string(),
    ];
    if let Some(at) = &proxy.admin_token {
        cmd.push(format!("--admin-token={}", &at));
    }
    if let Some(sk) = &proxy.store_key {
        cmd.push(format!("--store-key={}", &sk));
    }
    Config {
        image: Some(format!("{}:{}", img, version)),
        hostname: Some(domain(&proxy.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&proxy.name, ports, root_vol, Some(extra_vols)),
        cmd: Some(cmd),
        ..Default::default()
    }
}
