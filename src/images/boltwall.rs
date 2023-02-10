use super::*;
use crate::utils::{domain, exposed_ports, host_config};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BoltwallImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: String,
    pub links: Links,
}

impl BoltwallImage {
    pub fn new(name: &str, version: &str, port: &str, host: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            host: host.to_string(),
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

impl DockerHubImage for BoltwallImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-boltwall".to_string(),
        }
    }
}

pub fn navfiber(node: &Boltwallmage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let ports = vec![node.port.clone()];
    let labels = traefik::traefik_labels(&node.name, &node.host, &node.port);
    Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, None, None),
        labels: Some(labels),
        env: Some(vec![
            format!("PORT={}", node.port.clone()),
            format!("LND_TLS_CERT={}", ""),
            format!("LND_MACAROON={}", ""),
            format!("LND_SOCKET={}", ""),
            format!("BOLTWALL_MIN_AMOUNT=2"),
            format!("LIQUID_SERVER=https://liquid.sphinx.chat/"),
        ]),
        ..Default::default()
    }
}
