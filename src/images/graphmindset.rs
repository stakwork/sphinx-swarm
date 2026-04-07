use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct GraphMindsetImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
}

impl GraphMindsetImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("nav.{}", h));
        }
    }
}

#[async_trait]
impl DockerConfig for GraphMindsetImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(graphmindset(self))
    }
}

impl DockerHubImage for GraphMindsetImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "graphmindset".to_string(),
            root_volume: "/app/".to_string(),
        }
    }
}

fn graphmindset(node: &GraphMindsetImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = node.image();
    let root_vol = repo.root_volume;
    let ports = vec![node.port.clone()];

    let mut env = vec![];

    match getenv("NEXT_PUBLIC_API_URL") {
        Ok(api_url) => {
            env.push(format!("NEXT_PUBLIC_API_URL={}", api_url));
        }
        Err(_) => {
            log::debug!("NEXT_PUBLIC_API_URL not set");
        }
    }

    let mut c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, &root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };

    if let Some(host) = node.host.clone() {
        c.labels = Some(traefik_labels(&node.name, &host, "3000", false));
    }

    c
}
