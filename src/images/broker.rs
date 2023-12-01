use super::traefik::broker_traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BrokerImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub seed: String,
    pub mqtt_port: String,
    pub ws_port: Option<String>,
    pub logs: Option<String>,
    pub host: Option<String>,
    pub links: Links,
}

impl BrokerImage {
    pub fn new(
        name: &str,
        version: &str,
        network: &str,
        mqtt_port: &str,
        ws_port: Option<String>,
    ) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            seed: crate::secrets::hex_secret_32(),
            mqtt_port: mqtt_port.to_string(),
            ws_port: ws_port,
            links: vec![],
            host: None,
            logs: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn set_logs(&mut self, logs_string: &str) {
        self.logs = Some(logs_string.to_string())
    }
    pub fn set_seed(&mut self, seed: &str) {
        self.seed = seed.to_string();
    }
}

#[async_trait]
impl DockerConfig for BrokerImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(broker(self))
    }
}

impl DockerHubImage for BrokerImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-broker".to_string(),
        }
    }
}

fn broker(img: &BrokerImage) -> Config<String> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = "/root/.broker";

    let mut env = vec![format!("SEED={}", img.seed)];

    let mut ports = vec![img.mqtt_port.clone()];
    env.push(format!("BROKER_MQTT_PORT={}", &img.mqtt_port));
    if let Some(wsp) = &img.ws_port {
        ports.push(wsp.clone());
        env.push(format!("BROKER_WS_PORT={}", wsp));
    }
    if let Some(bl) = &img.logs {
        env.push(format!("BROKER_LOGS={}", bl));
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
        c.labels = Some(broker_traefik_labels(
            &img.name,
            &host,
            &img.mqtt_port,
            img.ws_port.as_deref(),
        ))
    }
    c
}
