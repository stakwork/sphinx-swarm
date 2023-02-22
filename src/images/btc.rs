use super::{DockerConfig, DockerHubImage, Repository};
use crate::config::{Clients, Node};
use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::secrets;
use crate::utils::{docker_domain_127, domain, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use bollard::Docker;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BtcImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub user: String,
    pub pass: String,
}

impl BtcImage {
    pub fn new(name: &str, version: &str, network: &str, user: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            user: user.to_string(),
            pass: secrets::random_word(12),
        }
    }
    pub fn set_password(&mut self, password: &str) {
        self.pass = password.to_string();
    }
    pub async fn connect_client(&self, clients: &mut Clients) {
        let btc_rpc_url = format!("http://{}", docker_domain_127(&self.name));
        match BitcoinRPC::new_and_create_wallet(&self, &btc_rpc_url, "18443").await {
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
        "18443".to_string(),
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
        format!("-rpcuser={}", node.user),
        format!("-rpcpassword={}", node.pass),
        format!("-rpcbind={}.sphinx", node.name),
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
    // "bitcoin" network is default is not specified
    if node.network != "bitcoin" {
        cmd.push(format!("-{}=1", node.network));
    }
    Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&node.name)),
        // user: user(),
        cmd: Some(cmd),
        host_config: host_config(&node.name, ports, root_vol, None),
        ..Default::default()
    }
}

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
