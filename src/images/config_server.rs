use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ConfigImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub regtest_tribe: String,
    pub regtest_router: String,
    pub regest_default_lsp: String,
    pub mainnet_tribe: String,
    pub mainnet_router: String,
    pub mainnet_default_lsp: String,
    pub host: Option<String>,
}

impl ConfigImage {
    pub fn new(
        name: &str,
        version: &str,
        port: &str,
        regtest_tribe: String,
        regtest_router: String,
        regest_default_lsp: String,
        mainnet_tribe: String,
        mainnet_router: String,
        mainnet_default_lsp: String,
    ) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            regtest_tribe,
            regtest_router,
            regest_default_lsp,
            mainnet_tribe,
            mainnet_router,
            mainnet_default_lsp,
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
}

#[async_trait]
impl DockerConfig for ConfigImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(config_server(self)?)
    }
}

impl DockerHubImage for ConfigImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-config".to_string(),
        }
    }
}

fn config_server(img: &ConfigImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = "/home";

    let ports = vec![img.port.clone()];

    let env = vec![
        format!("ROCKET_ADDRESS=0.0.0.0"),
        format!("ROCKET_PORT={}", img.port),
        format!("REGTEST_TRIBE={}", img.regtest_tribe),
        format!("REGTEST_ROUTER={}", img.regtest_router),
        format!("REGTEST_DEFAULT_LSP={}", img.regest_default_lsp),
        format!("MAINNET_TRIBE={}", img.mainnet_tribe),
        format!("MAINNET_ROUTER={}", img.mainnet_router),
        format!("MAINNET_DEFAULT_LSP={}", img.mainnet_default_lsp),
    ];

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };
    if let Some(host) = &img.host {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false))
    }
    Ok(c)
}
