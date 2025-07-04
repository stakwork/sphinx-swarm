use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct RedisImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub links: Links,
    pub host: Option<String>,
}

impl RedisImage {
    pub fn new(name: &str, version: &str) -> Self {
        // ports are hardcoded
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "6379".to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("redis.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for RedisImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(redis(self))
    }
}

impl DockerHubImage for RedisImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "library".to_string(),
            repo: "redis".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn redis(node: &RedisImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let env = vec![format!("PUBLIC_APP_NAME=SphinxChat")];

    let mut c = Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        env: Some(env),
        host_config: host_config(&name, ports, root_vol, None, None),
        ..Default::default()
    };
    if let Some(host) = &node.host {
        c.labels = Some(traefik_labels(&node.name, &host, &node.http_port, false))
    }
    c
}
