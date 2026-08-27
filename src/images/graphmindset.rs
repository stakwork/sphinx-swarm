use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct GraphMindsetImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
}

impl GraphMindsetImage {
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
            self.host = Some(format!("graph.{}", h));
        }
    }
}

#[async_trait]
impl DockerConfig for GraphMindsetImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(graphmindset(self))
    }
}

impl DockerHubImage for GraphMindsetImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "graphmindset".to_string(),
            root_volume: "/data/".to_string(),
        }
    }
}

fn graphmindset(node: &GraphMindsetImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = node.image();
    let root_vol = repo.root_volume;
    let ports = vec![node.port.clone()];

    let mut env = vec![
        format!("PORT={}", node.port),
    ];

    // GRAPH_MINDSET_API_URL wins; NEXT_PUBLIC_API_URL is still accepted so
    // existing swarm environments keep working without an edit.
    //
    // The resolved value is passed under both names. GraphMindset reads
    // GRAPH_MINDSET_API_URL from the live environment on each boot and injects
    // it into the served document — a NEXT_PUBLIC_* name cannot do that job,
    // because Next inlines anything with that prefix at build time, freezing
    // whatever was set when the image was built.
    let api_url = getenv("GRAPH_MINDSET_API_URL")
        .or_else(|_| getenv("NEXT_PUBLIC_API_URL"));

    match api_url {
        Ok(api_url) => {
            env.push(format!("GRAPH_MINDSET_API_URL={}", api_url));
            // Retained for images built before the runtime-config change.
            env.push(format!("NEXT_PUBLIC_API_URL={}", api_url));
        }
        Err(_) => {
            log::debug!("neither GRAPH_MINDSET_API_URL nor NEXT_PUBLIC_API_URL is set");
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

    if let Some(host) = node.host.clone() {
        c.labels = Some(traefik_labels(&node.name, &host, &node.port, false));
    }

    c
}
