use super::traefik::traefik_labels;
use super::*;
use crate::utils::{domain, exposed_ports, host_config, single_host_port_from_eighty};
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
        if let Some(h) = eh {
            self.host = Some(format!("nav.{}", h));
        }
    }
}

impl DockerHubImage for NavFiberImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-nav-fiber".to_string(),
        }
    }
}

pub fn navfiber(node: &NavFiberImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/usr/src/app/";
    let ports = vec![node.port.clone()];

    let mut c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None),
        env: None,
        ..Default::default()
    };
    // override the nginix port 80 -> 8001:80
    c.host_config.as_mut().unwrap().port_bindings = single_host_port_from_eighty(&node.port);

    if let Some(host) = node.host.clone() {
        // navfiber image uses nginx (port 80)
        let port_for_traefik = "80";
        // production tls extra domain
        c.labels = Some(traefik_labels(&node.name, &host, &port_for_traefik, false));
    }
    c
}
