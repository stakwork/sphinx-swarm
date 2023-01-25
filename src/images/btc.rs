use super::{DockerHubImage, Repository};
use crate::secrets;
use crate::utils::{domain, host_config};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
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
