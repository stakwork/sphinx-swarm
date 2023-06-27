use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

/*
CLN_MAINNET_BTC=http://evan:thepass@localhost:8332
*/

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct LssImage {
    pub name: String,
    pub version: String,
}

impl LssImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}

#[async_trait]
impl DockerConfig for LssImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(lss(self))
    }
}

impl DockerHubImage for LssImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-lss".to_string(),
        }
    }
}

fn lss(node: &LssImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/root/";
    let ports = vec!["55551".to_string()];

    Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None),
        env: None,
        ..Default::default()
    }
}

// vls lightning-storage-server Cross.toml
/*
[build]
pre-build = ["apt-get install protobuf-compiler -y"]
*/

// cross build --release --target x86_64-unknown-linux-musl --no-default-features --features crypt

// vls Dockerfile
/*
FROM alpine

COPY ./lightning-storage-server/target/x86_64-unknown-linux-musl/release/lssd ./lssd

CMD ["./lssd", "--interface", "0.0.0.0"]
*/

// docker build -t lss .

// docker tag lss sphinxlightning/sphinx-lss:0.0.2

// docker push sphinxlightning/sphinx-lss:0.0.2
