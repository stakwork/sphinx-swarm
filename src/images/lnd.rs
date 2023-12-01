use super::traefik::traefik_labels;
use super::*;
use crate::config::{Clients, Node};
use crate::conn::lnd::setup;
use crate::conn::lnd::utils::{dl_cert, try_unlock_lnd};
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct LndImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub rpc_port: String,
    pub peer_port: String,
    pub http_port: Option<String>,
    pub links: Links,
    pub unlock_password: String,
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumechanvalid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_syncer: Option<String>,
}
impl LndImage {
    pub fn new(name: &str, version: &str, network: &str, rpc_port: &str, peer_port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            rpc_port: rpc_port.to_string(),
            peer_port: peer_port.to_string(),
            http_port: None,
            assumechanvalid: None,
            pinned_syncer: None,
            links: vec![],
            unlock_password: secrets::random_word(12),
            host: None,
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn unlock_password(&mut self, up: &str) {
        self.unlock_password = up.to_string();
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
    pub async fn post_startup(&self, proj: &str, docker: &Docker) -> Result<()> {
        let cert_path = "/home/.lnd/tls.cert";
        let cert = dl_cert(docker, &self.name, cert_path).await?;
        try_unlock_lnd(&cert, proj, &self).await?;
        Ok(())
    }
    pub fn remove_client(&self, clients: &mut Clients) {
        clients.lnd.remove(&self.name);
    }
    pub async fn connect_client(
        &self,
        clients: &mut Clients,
        docker: &Docker,
        nodes: &Vec<Node>,
    ) -> Result<()> {
        sleep(1).await;
        let (client, test_mine_addy) = setup::lnd_clients(docker, self).await?;
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let btc = li.find_btc().context("BTC required for LND")?;
        setup::test_mine_if_needed(test_mine_addy, &btc.name, clients);
        clients.lnd.insert(self.name.clone(), client);
        Ok(())
    }
}

#[async_trait]
impl DockerConfig for LndImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let btc = li.find_btc().context("BTC required for LND")?;
        Ok(lnd(&self, &btc))
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

pub fn to_lnd_network(n: &str) -> &'static str {
    match n {
        "bitcoin" => "mainnet",
        "simnet" => "simnet",
        "regtest" => "regtest",
        _ => "regtest",
    }
}

fn lnd(lnd: &LndImage, btc: &btc::BtcImage) -> Config<String> {
    let network = to_lnd_network(lnd.network.as_str());
    let repo = lnd.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let mut ports = vec![lnd.peer_port.to_string(), lnd.rpc_port.clone()];
    // let home_dir = std::env::var("HOME").unwrap_or("/home".to_string());
    let root_vol = "/home/.lnd";
    // println!("LND LINKS {:?}", links);
    let btc_domain = domain(&btc.name);
    let mut cmd = vec![
        format!("--bitcoin.active"),
        format!("--bitcoin.node=bitcoind"),
        format!("--lnddir={}", root_vol),
        format!("--bitcoin.{}", network),
        format!("--listen=0.0.0.0:{}", &lnd.peer_port),
        format!("--rpclisten=0.0.0.0:{}", &lnd.rpc_port),
        format!("--tlsextradomain={}", domain(&lnd.name)),
        format!("--alias={}", &lnd.name),
        format!("--bitcoind.rpchost={}:18443", &btc_domain),
        format!("--bitcoind.zmqpubrawblock=tcp://{}:28332", &btc_domain),
        format!("--bitcoind.zmqpubrawtx=tcp://{}:28333", &btc_domain),
        format!("--bitcoin.basefee=0"),
        format!("--bitcoin.feerate=3"),
        format!("--bitcoin.defaultchanconfs=2"),
        format!("--accept-keysend"),
        format!("--accept-amp"),
        format!("--db.bolt.auto-compact"),
    ];
    if let Some(u) = &btc.user {
        if let Some(p) = &btc.pass {
            cmd.push(format!("--bitcoind.rpcuser={}", u));
            cmd.push(format!("--bitcoind.rpcpass={}", p));
        }
    }
    if let Some(acv) = lnd.assumechanvalid {
        if acv {
            log::info!("[lnd]: --routing.assumechanvalid");
            cmd.push("--routing.assumechanvalid".to_string());
        }
    }
    if let Some(ps) = &lnd.pinned_syncer {
        log::info!("[lnd]: --gossip.pinned-syncers={}", ps);
        cmd.push(format!("--gossip.pinned-syncers={}", ps));
    }
    if let Some(hp) = lnd.http_port.clone() {
        ports.push(hp.clone());
        let rest_host = "0.0.0.0";
        cmd.push(format!("--restlisten={}:{}", rest_host, hp).to_string());
    }
    let mut c = Config {
        image: Some(format!("{}:{}", img, lnd.version).to_string()),
        hostname: Some(domain(&lnd.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&lnd.name, ports, root_vol, None, None),
        ..Default::default()
    };
    if let Ok(_) = std::env::var("DEBUG") {
        cmd.push(format!("--debuglevel=debug"));
    }
    if let Some(host) = lnd.host.clone() {
        c.labels = Some(traefik_labels(&lnd.name, &host, &lnd.peer_port, false));
        // production tls extra domain
        cmd.push(format!("--tlsextradomain={}", &host));
    }
    c.cmd = Some(cmd);
    c
}

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
