use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ChromeImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    pub host: Option<String>,
}

/*

- place: Internal
  type: Chrome
  name: chrome
  version: latest
  port: '8080'
  host: chrome.swarm38.sphinx.chat

*/

impl ChromeImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
}

// with ndeo4j
#[async_trait]
impl DockerConfig for ChromeImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(chrome(self)?)
    }
}

impl DockerHubImage for ChromeImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::Ghcr,
            org: "evanfeenstra".to_string(),
            repo: "chrome-playwright".to_string(),
            root_volume: "/root".to_string(),
        }
    }
}

fn chrome(img: &ChromeImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let env = vec![format!("PORT={}", img.port)];

    let c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };
    Ok(c)
}
