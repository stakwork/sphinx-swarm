use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::broker::BrokerImage;
use crate::images::jarvis::JarvisImage;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct RunnerImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
}

impl RunnerImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            host: None,
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
}

#[async_trait]
impl DockerConfig for RunnerImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let broker = li.find_broker().context("Runner: No Broker")?;
        let jarvis = li.find_jarvis().context("Runner: No Jarvis")?;
        Ok(runner(self, &broker, &jarvis)?)
    }
}

impl DockerHubImage for RunnerImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-runner".to_string(),
            root_volume: "/home".to_string(),
        }
    }
}

fn runner(img: &RunnerImage, broker: &BrokerImage, jarvis: &JarvisImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let env = vec![
        format!(
            "BROKER_URL=http://{}:{}",
            domain(&broker.name),
            broker.mqtt_port
        ),
        format!("JARVIS_URL=http://{}:{}", domain(&jarvis.name), jarvis.port),
        format!("ROCKET_ADDRESS=0.0.0.0"),
        format!("ROCKET_PORT={}", img.port),
    ];

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
