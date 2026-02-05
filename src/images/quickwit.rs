use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{docker_domain, domain, exposed_ports, host_config, make_reqwest_client};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Maximum storage in MB before cleanup triggers (default 32GB)
const MAX_STORAGE_MB: u64 = 32_000;

fn get_max_storage_mb() -> u64 {
    std::env::var("QUICKWIT_MAX_STORAGE_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(MAX_STORAGE_MB)
}

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
        let index_exists = match client.get(&check_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                log::info!("=> quickwit logs index already exists");
                true
            }
            _ => false,
        };

        if index_exists {
            // Spawn cleanup task and return
            self.spawn_cleanup_task();
            return Ok(());
        }

        log::info!("=> creating quickwit logs index...");

        // Index config matching Vercel log drain format
        // Vercel sends: id, deploymentId, source, host, timestamp (unix ms), projectId, level, message, etc.
        let index_config = serde_json::json!({
            "version": "0.7",
            "index_id": "logs",
            "doc_mapping": {
                "mode": "dynamic",
                "field_mappings": [
                    {
                        "name": "timestamp",
                        "type": "datetime",
                        "input_formats": ["unix_timestamp"],
                        "output_format": "unix_timestamp_millis",
                        "fast_precision": "milliseconds",
                        "fast": true
                    },
                    {"name": "id", "type": "text", "tokenizer": "raw"},
                    {"name": "deploymentId", "type": "text", "tokenizer": "raw"},
                    {"name": "projectId", "type": "text", "tokenizer": "raw"},
                    {"name": "projectName", "type": "text"},
                    {"name": "source", "type": "text", "tokenizer": "raw"},
                    {"name": "level", "type": "text", "tokenizer": "raw"},
                    {"name": "message", "type": "text"},
                    {"name": "host", "type": "text", "tokenizer": "raw"},
                    {"name": "path", "type": "text"},
                    {"name": "statusCode", "type": "i64"},
                    {"name": "requestId", "type": "text", "tokenizer": "raw"},
                    {"name": "environment", "type": "text", "tokenizer": "raw"}
                ],
                "timestamp_field": "timestamp"
            },
            "indexing_settings": {
                "commit_timeout_secs": 10
            },
            "retention": {
                "period": "7 days",
                "schedule": "daily"
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

        // Spawn background cleanup task
        self.spawn_cleanup_task();

        Ok(())
    }

    fn spawn_cleanup_task(&self) {
        let quickwit_host = docker_domain(&self.name);
        let http_port = self.http_port.clone();

        tokio::spawn(async move {
            // Check every hour
            let check_interval = Duration::from_secs(3600);

            // Initial delay to let Quickwit fully start, then check immediately
            tokio::time::sleep(Duration::from_secs(10)).await;

            log::info!(
                "=> quickwit cleanup: starting first check against {}:{}",
                quickwit_host,
                http_port
            );

            loop {
                if let Err(e) = cleanup_if_over_limit(&quickwit_host, &http_port).await {
                    log::warn!("=> quickwit cleanup error: {:?}", e);
                }

                tokio::time::sleep(check_interval).await;
            }
        });

        let max_mb = get_max_storage_mb();
        log::info!(
            "=> quickwit storage cleanup task started (max {}MB)",
            max_mb
        );
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

async fn cleanup_if_over_limit(quickwit_host: &str, http_port: &str) -> Result<()> {
    let client = make_reqwest_client();
    let max_mb = get_max_storage_mb();
    let max_bytes = max_mb * 1024 * 1024;

    // Get all splits (contains size info)
    let splits_url = format!(
        "http://{}:{}/api/v1/indexes/logs/splits",
        quickwit_host, http_port
    );
    let resp = match client.get(&splits_url).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("=> quickwit cleanup: failed to fetch splits: {:?}", e);
            return Ok(());
        }
    };

    if !resp.status().is_success() {
        log::warn!("=> quickwit cleanup: splits API returned {}", resp.status());
        return Ok(()); // Index might not exist yet
    }

    let splits_info: serde_json::Value = resp.json().await?;
    let splits = splits_info
        .get("splits")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();

    // Calculate total size from splits
    let current_bytes: u64 = splits
        .iter()
        .filter_map(|s| s.get("uncompressed_docs_size_in_bytes")?.as_u64())
        .sum();

    let current_mb = current_bytes / (1024 * 1024);
    log::info!(
        "=> quickwit storage check: {}MB / {}MB ({} splits)",
        current_mb,
        max_mb,
        splits.len()
    );

    if current_bytes <= max_bytes {
        return Ok(());
    }

    log::info!(
        "=> quickwit storage over limit ({}MB > {}MB), cleaning up oldest splits...",
        current_mb,
        max_mb
    );

    if splits.is_empty() {
        return Ok(());
    }

    // Sort splits by time_range start (oldest first)
    let mut splits_with_time: Vec<(String, i64, u64)> = splits
        .iter()
        .filter_map(|s| {
            let split_id = s.get("split_id")?.as_str()?.to_string();
            let time_range = s.get("time_range")?;
            let start = time_range.get("start")?.as_i64()?;
            let size = s.get("uncompressed_docs_size_in_bytes")?.as_u64()?;
            Some((split_id, start, size))
        })
        .collect();

    splits_with_time.sort_by_key(|(_, start, _)| *start);

    // Calculate how much to delete
    let bytes_to_delete = current_bytes - max_bytes;
    let mut deleted_bytes: u64 = 0;
    let mut splits_to_delete: Vec<String> = vec![];

    // Never delete all splits - keep at least one
    let max_to_delete = splits_with_time.len().saturating_sub(1);

    for (split_id, _, size) in splits_with_time.into_iter().take(max_to_delete) {
        if deleted_bytes >= bytes_to_delete {
            break;
        }
        splits_to_delete.push(split_id);
        deleted_bytes += size;
    }

    if splits_to_delete.is_empty() {
        return Ok(());
    }

    log::info!(
        "=> deleting {} oldest splits ({}MB)",
        splits_to_delete.len(),
        deleted_bytes / (1024 * 1024)
    );

    // Mark splits for deletion
    let mark_url = format!(
        "http://{}:{}/api/v1/indexes/logs/splits/mark-for-deletion",
        quickwit_host, http_port
    );
    let resp = client
        .put(&mark_url)
        .json(&serde_json::json!({ "split_ids": splits_to_delete }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        log::warn!("=> failed to mark splits for deletion: {}", body);
        return Ok(());
    }

    // Quickwit will garbage collect marked splits automatically
    log::info!("=> quickwit cleanup: marked splits for deletion, GC will run automatically");

    Ok(())
}
