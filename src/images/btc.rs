use super::traefik::traefik_labels;
use super::{DockerConfig, DockerHubImage, Repository};
use crate::config::{Clients, Node};
use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::utils::{docker_domain_127, domain, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use bollard::Docker;
use serde::{Deserialize, Serialize};

const RPC_PORT: &str = "18443";

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BtcImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub host: Option<String>,
}

impl BtcImage {
    pub fn new(name: &str, version: &str, network: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            user: None,
            pass: None,
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("bitcoind.{}", h));
        }
    }
    pub fn set_user_password(&mut self, user: &str, password: &str) {
        self.user = Some(user.to_string());
        self.pass = Some(password.to_string());
    }
    pub async fn post_client(&self, clients: &Clients) -> Result<()> {
        let client = clients
            .bitcoind
            .get(&self.name)
            .context("no bitcoind client")?;
        client.load_wallet()?;
        Ok(())
    }
    pub fn remove_client(&self, clients: &mut Clients) {
        clients.bitcoind.remove(&self.name);
    }
    pub async fn connect_client(&self, clients: &mut Clients) {
        let btc_rpc_url = format!("http://{}", docker_domain_127(&self.name));
        match BitcoinRPC::new_and_create_wallet(&self, &btc_rpc_url, RPC_PORT).await {
            Ok(client) => {
                clients.bitcoind.insert(self.name.clone(), client);
            }
            Err(e) => log::warn!("BitcoinRPC error: {:?}", e),
        };
        sleep(1).await;
    }
}

#[async_trait]
impl DockerConfig for BtcImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(btc(self))
    }
}

impl DockerHubImage for BtcImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "lncm".to_string(),
            repo: "bitcoind".to_string(),
        }
    }
}

pub fn btc(node: &BtcImage) -> Config<String> {
    let ports = vec![
        RPC_PORT.to_string(),
        "28332".to_string(),
        "28333".to_string(),
        "8333".to_string(),
        "8332".to_string(),
    ];
    // let image = "ruimarinho/bitcoin-core";
    let repo = node.repo();
    let image = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/data/.bitcoin";
    let mut cmd = vec![
        format!("-rpcbind={}", domain(&node.name)),
        "-rpcallowip=0.0.0.0/0".to_string(),
        "-rpcport=18443".to_string(),
        "-server=1".to_string(),
        "-txindex=1".to_string(),
        "-fallbackfee=0.0002".to_string(),
        "-zmqpubrawblock=tcp://0.0.0.0:28332".to_string(),
        "-zmqpubrawtx=tcp://0.0.0.0:28333".to_string(),
        "-rpcbind=127.0.0.1".to_string(),
        "-maxconnections=10".to_string(),
        "-minrelaytxfee=0.00000000".to_string(),
        "-incrementalrelayfee=0.00000010".to_string(),
    ];
    if let Some(u) = &node.user {
        if let Some(p) = &node.pass {
            cmd.push(format!("-rpcuser={}", u));
            cmd.push(format!("-rpcpassword={}", p));
        }
    }
    // "bitcoin" network is default is not specified
    if node.network != "bitcoin" {
        cmd.push(format!("-{}=1", node.network));
    }
    let mut c = Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&node.name)),
        // user: user(),
        cmd: Some(cmd),
        host_config: host_config(&node.name, ports, root_vol, None, None),
        ..Default::default()
    };
    if let Some(host) = node.host.clone() {
        // production tls extra domain
        c.labels = Some(traefik_labels(&node.name, &host, RPC_PORT, false));
    }
    c
}

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
