use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::neo4j::Neo4jImage;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct StakgraphImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    pub host: Option<String>,
}

impl StakgraphImage {
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
impl DockerConfig for StakgraphImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let neo4j = li.find_neo4j().context("Stakgraph: No Neo4j")?;
        Ok(stakgraph(self, &neo4j)?)
    }
}

impl DockerHubImage for StakgraphImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "stakwork".to_string(),
            repo: "stakgraph".to_string(),
            root_volume: "/root".to_string(),
        }
    }
}

fn stakgraph(
    img: &StakgraphImage,
    neo4j: &Neo4jImage,
) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("ghcr.io/{}/{}", repo.org, repo.repo);

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("PORT={}", img.port),
        format!("NEO4J_URI=bolt://{}:{}", domain(&neo4j.name), neo4j.bolt_port),
        format!("NEO4J_PASSWORD={}", neo4j.password),
    ];
    if let Ok(github_request_token) = getenv("GITHUB_REQUEST_TOKEN") {
        env.push(format!("PAT={}", github_request_token))
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
