use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{docker_domain, domain, exposed_ports, host_config, make_reqwest_client};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct QuickwitImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub grpc_port: String,
    pub links: Links,
    pub host: Option<String>,
}

impl QuickwitImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "7280".to_string(),
            grpc_port: "7281".to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("quickwit.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub async fn post_startup(&self) -> Result<()> {
        // Wait for Quickwit to be ready
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let client = make_reqwest_client();
        let quickwit_host = docker_domain(&self.name);
        let url = format!("http://{}:{}/api/v1/indexes", quickwit_host, self.http_port);

        // Check if logs index already exists
        let check_url = format!(
            "http://{}:{}/api/v1/indexes/logs",
            quickwit_host, self.http_port
        );
        match client.get(&check_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                log::info!("=> quickwit logs index already exists");
                return Ok(());
            }
            _ => {}
        }

        log::info!("=> creating quickwit logs index...");

        let index_config = serde_json::json!({
            "version": "0.7",
            "index_id": "logs",
            "doc_mapping": {
                "field_mappings": [
                    {"name": "timestamp", "type": "datetime", "fast": true},
                    {"name": "message", "type": "text"},
                    {"name": "level", "type": "text"},
                    {"name": "source", "type": "text"},
                    {"name": "service", "type": "text"}
                ],
                "timestamp_field": "timestamp"
            },
            "indexing_settings": {
                "commit_timeout_secs": 10
            }
        });

        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&index_config)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    log::info!("=> quickwit logs index created successfully");
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    log::warn!(
                        "=> failed to create quickwit logs index: {} - {}",
                        status,
                        body
                    );
                }
            }
            Err(e) => {
                log::warn!("=> failed to create quickwit logs index: {:?}", e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl DockerConfig for QuickwitImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(quickwit(self))
    }
}

impl DockerHubImage for QuickwitImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "quickwit".to_string(),
            repo: "quickwit".to_string(),
            root_volume: "/quickwit/qwdata".to_string(),
        }
    }
}

fn quickwit(node: &QuickwitImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone(), node.grpc_port.clone()];

    let mut c = Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None, None),
        cmd: Some(vec!["run".to_string()]),
        ..Default::default()
    };
    if let Some(host) = &node.host {
        c.labels = Some(traefik_labels(&node.name, &host, &node.http_port, false))
    }
    c
}
