use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::broker::BrokerImage;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MixerImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub no_lightning: Option<bool>,
    pub no_mqtt: Option<bool>,
    pub host: Option<String>,
    pub links: Links,
}

impl MixerImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            no_lightning: None,
            no_mqtt: None,
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
    pub fn set_no_lightning(&mut self) {
        self.no_lightning = Some(true)
    }
    pub fn set_no_mqtt(&mut self) {
        self.no_mqtt = Some(true)
    }
}

#[async_trait]
impl DockerConfig for MixerImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let broker = li.find_broker().context("Mixer: No Broker")?;
        Ok(mixer(self, &broker))
    }
}

impl DockerHubImage for MixerImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-mixer".to_string(),
        }
    }
}

fn mixer(img: &MixerImage, broker: &BrokerImage) -> Config<String> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = "/usr/src/data";

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("SEED={}", broker.seed),
        format!("ROCKET_ADDRESS=0.0.0.0"),
        format!("ROCKET_PORT={}", img.port),
    ];
    if let Some(nl) = img.no_lightning {
        if nl {
            env.push("NO_LIGHTNING=true".to_string());
        }
    }
    if let Some(nl) = img.no_mqtt {
        if nl {
            env.push("NO_MQTT=true".to_string());
        }
    }

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, None),
        env: Some(env),
        ..Default::default()
    };
    if let Some(host) = &img.host {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false))
    }
    c
}
