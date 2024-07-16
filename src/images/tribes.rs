use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::broker::BrokerImage;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TribesImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub port: String,
    pub host: Option<String>,
    pub tribes_host: Option<String>, // for testing
    pub links: Links,
    pub log_level: Option<String>,
    pub initial_routing_nodes: Option<String>, // fetch nodes at first
}

impl TribesImage {
    pub fn new(name: &str, version: &str, network: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            port: port.to_string(),
            links: vec![],
            host: None,
            tribes_host: getenv("TRIBES_HOST").ok(),
            log_level: None,
            initial_routing_nodes: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
            self.tribes_host = self.host.clone();
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn set_log_level(&mut self, log_level: &str) {
        self.log_level = Some(log_level.to_string())
    }
    pub fn set_initial_routing_nodes(&mut self, irns: &str) {
        self.initial_routing_nodes = Some(irns.to_string())
    }
}

#[async_trait]
impl DockerConfig for TribesImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let broker = li.find_broker().context("Tribes: No Broker")?;
        Ok(tribes(self, &broker)?)
    }
}

impl DockerHubImage for TribesImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-tribes-v2".to_string(),
            root_volume: "/home".to_string(),
        }
    }
}

fn tribes(img: &TribesImage, broker: &BrokerImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("SEED={}", broker.seed),
        format!("DB_PATH=/home/tribes"),
        format!("ROCKET_ADDRESS=0.0.0.0"),
        format!("ROCKET_PORT={}", img.port),
    ];

    if let Some(th) = &img.tribes_host {
        env.push(format!("HOST={}", th));
    }

    let bu = format!("{}:{}", domain(&broker.name), broker.mqtt_port);
    env.push(format!("BROKER_URL={}", bu));

    if let Some(ll) = &img.log_level {
        env.push(format!("RUST_LOG={}", ll));
    }

    if let Some(irns) = &img.initial_routing_nodes {
        env.push(format!("INITIAL_ROUTING_NODES={}", irns));
    }

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
