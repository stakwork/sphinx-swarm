use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct EgressImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub links: Links,
    pub host: Option<String>,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
}

impl EgressImage {
    pub fn new(name: &str, version: &str, livekit_api_key: &str, livekit_api_secret: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "9090".to_string(),
            links: vec![],
            host: None,
            livekit_url: "http://livekit:7880".to_string(),
            livekit_api_key: livekit_api_key.to_string(),
            livekit_api_secret: livekit_api_secret.to_string(),
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            // self.host = Some(format!("egress.{}", h));
            self.livekit_url = format!("http://livekit.{}", h);
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for EgressImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(egress(self))
    }
}

impl DockerHubImage for EgressImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "livekit".to_string(),
            repo: "egress".to_string(),
            root_volume: "/out".to_string(),
        }
    }
}

fn egress(node: &EgressImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let config_json = format!(
        r#"{{
  "api_key": "{}",
  "api_secret": "{}",
  "ws_url": "ws://{}",
  "redis": {{
    "address": "redis:6379"
  }},
  "log_level": "info",
  "template_base": "/out"
}}"#,
        node.livekit_api_key, 
        node.livekit_api_secret, 
        node.livekit_url.replace("http://", "")
    );

    let env = vec![
        format!("EGRESS_CONFIG_BODY={}", config_json),
    ];

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