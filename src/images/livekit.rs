use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct LivekitImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub rtc_port: String,
    pub turn_port: String,
    pub links: Links,
    pub host: Option<String>,
    pub api_key: String,
    pub api_secret: String,
}

impl LivekitImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "7880".to_string(),
            rtc_port: "7881".to_string(),
            turn_port: "3478".to_string(),
            links: vec![],
            host: None,
            api_key: secrets::random_word(32),
            api_secret: secrets::random_word(32),
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Ok(livekit_domain) = std::env::var("LIVEKIT_DOMAIN") {
            self.host = Some(livekit_domain);
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for LivekitImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(livekit(self))
    }
}

impl DockerHubImage for LivekitImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "livekit".to_string(),
            repo: "livekit-server".to_string(),
            root_volume: "/livekit".to_string(),
        }
    }
}

fn livekit(node: &LivekitImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![
        node.http_port.clone(),
        node.rtc_port.clone(),
        node.turn_port.clone(),
    ];

    // Get external IP from environment or use default
    let external_ip = std::env::var("EXTERNAL_IP").unwrap_or_else(|_| "".to_string());
    
    let config_json = format!(
        r#"{{
  "port": {},
  "rtc": {{
    "tcp_port": {},
    "port_range_start": 50000,
    "port_range_end": 50100,
    "use_external_ip": false,
    "node_ip": "{}",
    "allow_tcp_fallback": true
  }},
  "redis": {{
    "address": "redis:6379"
  }},
  "keys": {{
    "{}": "{}"
  }},
  "room": {{
    "auto_create": true
  }},
  "logging": {{
    "level": "info"
  }},
  "turn": {{
    "enabled": false
  }}
}}"#,
        node.http_port, node.rtc_port, external_ip, node.api_key, node.api_secret
    );

    let env = vec![
        format!("LIVEKIT_CONFIG={}", config_json),
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
        c.labels = Some(traefik_labels(&node.name, &host, &node.http_port, true)) // Enable websockets for LiveKit
    }
    c
}