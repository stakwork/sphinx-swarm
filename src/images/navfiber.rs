use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{
    domain, exposed_ports, host_config, is_using_port_based_ssl, single_host_port_from, getenv,
};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct NavFiberImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
}

impl NavFiberImage {
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
        self.links = strarr(links)
    }
    pub fn host(&mut self, eh: Option<String>) {
        // if let Some(h) = eh {
        //     self.host = Some(format!("nav.{}", h));
        // }
    }
}

#[async_trait]
impl DockerConfig for NavFiberImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(navfiber(self))
    }
}

impl DockerHubImage for NavFiberImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "sphinx-nav-fiber".to_string(),
            root_volume: "/usr/src/app/".to_string(),
        }
    }
}

fn navfiber(node: &NavFiberImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = node.image();
    let root_vol = repo.root_volume;
    let ports = vec![node.port.clone()];

    let mut env = vec![];

    // Pass BOLTWALL_URL to navfiber container (runtime env var)
    match getenv("BOLTWALL_URL") {
        Ok(boltwall_url) => {
            env.push(format!("BOLTWALL_URL={}", boltwall_url));
        }
        Err(_) => {
            log::debug!("BOLTWALL_URL not set");
        }
    }

    match getenv("STAKWORK_WEBSOCKET_URL") {
        Ok(env_var) => {
            env.push(format!("STAKWORK_WEBSOCKET_URL={}", env_var));
        }
        Err(_) => {
            log::debug!("STAKWORK_WEBSOCKET_URL not set");
        }
    }

    let mut c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, &root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };
    let inner_port = "80";
    // override the nginix port 80 -> 8000:80
    if !is_using_port_based_ssl() {
        c.host_config.as_mut().unwrap().port_bindings =
            single_host_port_from(&node.port, inner_port);
    }

    if let Some(host) = node.host.clone() {
        // navfiber image uses nginx (port 80)
        // production tls extra domain
        c.labels = Some(traefik_labels(&node.name, &host, inner_port, false));
    }
    
    c
}
