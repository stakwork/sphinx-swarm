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

pub fn btc(project: &str, node: &BtcImage) -> Config<String> {
    let ports = vec![
        "18443".to_string(),
        "28332".to_string(),
        "28333".to_string(),
    ];
    // let image = "ruimarinho/bitcoin-core";
    let repo = node.repo();
    let image = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/data/.bitcoin";
    Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&node.name)),
        // user: user(),
        cmd: Some(vec![
            format!("-{}=1", node.network),
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
        ]),
        host_config: host_config(project, &node.name, ports, root_vol, None, None),
        ..Default::default()
    }
}
