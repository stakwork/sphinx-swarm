use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MeetImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub links: Links,
    pub host: Option<String>,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
}

impl MeetImage {
    pub fn new(name: &str, version: &str, livekit_api_key: &str, livekit_api_secret: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "3000".to_string(),
            links: vec![],
            host: None,
            livekit_url: "".to_string(),
            livekit_api_key: livekit_api_key.to_string(),
            livekit_api_secret: livekit_api_secret.to_string(),
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(h.clone());
            self.livekit_url = format!("wss://{}", h);
        } else {
            // Use external IP for LiveKit connection with HTTPS
            let external_ip = std::env::var("EXTERNAL_IP").unwrap_or_else(|_| "".to_string());
            let no_ip_domain = std::env::var("MEET_DOMAIN").unwrap_or_else(|_| format!("{}:7880", external_ip));
            self.livekit_url = format!("wss://{}", no_ip_domain);
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for MeetImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(meet(self))
    }
}

impl DockerHubImage for MeetImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "sphinx-livekit".to_string(),
            root_volume: "/app".to_string(),
        }
    }
}

fn meet(node: &MeetImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let env = vec![
        format!("LIVEKIT_URL={}", node.livekit_url),
        format!("LIVEKIT_API_KEY={}", node.livekit_api_key),
        format!("LIVEKIT_API_SECRET={}", node.livekit_api_secret),
        "NEXTAUTH_SECRET=your-secret-key".to_string(),
        // Use HTTPS URL if host is configured
        if let Some(ref host) = node.host {
            format!("NEXTAUTH_URL=https://{}", host)
        } else {
            format!("NEXTAUTH_URL=http://{}:{}", name, node.http_port)
        },
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