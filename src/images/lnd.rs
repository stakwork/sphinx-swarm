use super::*;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config, user};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LndImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub port: String,
    pub http_port: Option<String>,
    pub links: Links,
    pub unlock_password: String,
}
impl LndImage {
    pub fn new(name: &str, version: &str, network: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            port: port.to_string(),
            http_port: None,
            links: vec![],
            unlock_password: secrets::random_word(12),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn unlock_password(&mut self, up: &str) {
        self.unlock_password = up.to_string();
    }
}
impl DockerHubImage for LndImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "lightninglabs".to_string(),
            repo: "lnd".to_string(),
        }
    }
}

pub fn lnd(project: &str, lnd: &LndImage, btc: &btc::BtcImage) -> Config<String> {
    let network = match lnd.network.as_str() {
        "bitcoin" => "mainnet",
        "simnet" => "simnet",
        "regtest" => "regtest",
        _ => "regtest",
    };
    let repo = lnd.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let peering_port = "9735";
    let mut ports = vec![peering_port.to_string(), lnd.port.clone()];
    // let home_dir = std::env::var("HOME").unwrap_or("/home".to_string());
    let root_vol = "/home/.lnd";
    let links = Some(vec![domain(&btc.name)]);
    let btc_domain = domain(&btc.name);
    let mut cmd = vec![
        format!("--debuglevel=debug"),
        format!("--bitcoin.active"),
        format!("--bitcoin.node=bitcoind"),
        format!("--lnddir={}", root_vol),
        format!("--bitcoin.{}", network),
        format!("--rpclisten=0.0.0.0:{}", &lnd.port),
        format!("--tlsextradomain={}.sphinx", lnd.name),
        format!("--alias={}", &lnd.name),
        format!("--bitcoind.rpcuser={}", &btc.user),
        format!("--bitcoind.rpcpass={}", &btc.pass),
        format!("--bitcoind.rpchost={}", &btc_domain),
        // format!("--bitcoind.rpcpolling"),
        format!("--bitcoind.zmqpubrawblock=tcp://{}:28332", &btc_domain),
        format!("--bitcoind.zmqpubrawtx=tcp://{}:28333", &btc_domain),
        format!("--bitcoin.basefee=0"),
        format!("--bitcoin.feerate=3"),
        format!("--bitcoin.defaultchanconfs=2"),
        format!("--accept-keysend"),
        format!("--accept-amp"),
        format!("--db.bolt.auto-compact"),
    ];
    if let Some(hp) = lnd.http_port.clone() {
        ports.push(hp.clone());
        let rest_host = "0.0.0.0";
        cmd.push(format!("--restlisten={}:{}", rest_host, hp).to_string());
    }
    Config {
        image: Some(format!("{}:{}", img, lnd.version).to_string()),
        hostname: Some(domain(&lnd.name)),
        user: user(),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(project, &lnd.name, ports, root_vol, None, links),
        cmd: Some(cmd),
        ..Default::default()
    }
}
