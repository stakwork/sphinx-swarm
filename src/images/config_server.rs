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
    pub host: Option<String>,
}

impl ConfigImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
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

fn getenv_(name: &str) -> String {
    std::env::var(name).unwrap_or("".to_string())
}

fn config_server(img: &ConfigImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = "/home";

    let ports = vec![img.port.clone()];

    let env = vec![
        format!("ROCKET_ADDRESS=0.0.0.0"),
        format!("ROCKET_PORT={}", img.port),
        format!("REGTEST_TRIBE={}", getenv_("REGTEST_TRIBE")),
        format!("REGTEST_TRIBE_HOST={}", getenv_("REGTEST_TRIBE_HOST")),
        format!("REGTEST_ROUTER={}", getenv_("REGTEST_ROUTER")),
        format!("REGTEST_DEFAULT_LSP={}", getenv_("REGTEST_DEFAULT_LSP")),
        format!("REGTEST_LSPS={}", getenv_("REGTEST_LSPS")),
        format!("MAINNET_TRIBE={}", getenv_("MAINNET_TRIBE")),
        format!("MAINNET_TRIBE_HOST={}", getenv_("MAINNET_TRIBE_HOST")),
        format!("MAINNET_ROUTER={}", getenv_("MAINNET_ROUTER")),
        format!("MAINNET_DEFAULT_LSP={}", getenv_("MAINNET_DEFAULT_LSP")),
        format!("MAINNET_LSPS={}", getenv_("MAINNET_LSPS")),
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
