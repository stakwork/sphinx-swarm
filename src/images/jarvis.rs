use super::*;
use crate::utils::{domain, exposed_ports, host_config};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct JarvisBackendImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
}

impl JarvisBackendImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

impl DockerHubImage for JarvisBackendImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-jarvis-backend".to_string(),
        }
    }
}

pub fn jarvis(node: &JarvisBackendImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/data/jarvis";
    let ports = vec![node.port.clone()];
    Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None),
        env: Some(vec![
            format!("NEO4J_URI=neo4j://neo4j.sphinx:7687"),
            format!("NEO4J_USER=neo4j"),
            format!("NEO4J_PASS=test"),
        ]),
        ..Default::default()
    }
}
