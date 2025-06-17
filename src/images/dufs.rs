use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct DufsImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub files_dir: String,
    pub host: Option<String>,
    pub links: Links,
}

impl DufsImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            files_dir: "/files".to_string(),
            host: None,
            links: vec![],
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
impl DockerConfig for DufsImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        // let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        Ok(dufs(self)?)
    }
}

impl DockerHubImage for DufsImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "dufs".to_string(),
            root_volume: self.files_dir.to_string(),
        }
    }
}

fn dufs(img: &DufsImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let env = vec![
        format!("DUFS_PORT={}", img.port),
        format!("DUFS_ALLOW_SEARCH=true"),
    ];

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, None, None),
        env: Some(env),
        entrypoint: Some(vec!["/usr/src/dufs".to_string(), img.files_dir.clone()]),
        ..Default::default()
    };
    if let Some(host) = &img.host {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false))
    }
    Ok(c)
}
