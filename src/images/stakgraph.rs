use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::boltwall::BoltwallImage;
use crate::images::neo4j::Neo4jImage;
use crate::images::traefik::navfiber_boltwall_shared_host;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_log: Option<String>, // for debugging
}

/*

- place: Internal
  type: Stakgraph
  name: stakgraph
  version: latest
  port: '7799'
  host: stakgraph.swarm38.sphinx.chat
  links:
  - boltwall
  - neo4j

*/

impl StakgraphImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            rust_log: None,
            host: None,
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(shared_host) = navfiber_boltwall_shared_host() {
            self.host = Some(format!("{}.{}", self.name, shared_host))
        } else {
            if let Some(h) = eh {
                self.host = Some(format!("{}.{}", self.name, h));
            }
        }
    }
}

// with ndeo4j
#[async_trait]
impl DockerConfig for StakgraphImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let neo4j = li.find_neo4j().context("Stakgraph: No Neo4j")?;
        let boltwall = li.find_boltwall();
        Ok(stakgraph(self, &neo4j, &boltwall)?)
    }
}

impl DockerHubImage for StakgraphImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::Ghcr,
            org: "stakwork".to_string(),
            repo: "stakgraph-standalone".to_string(),
            root_volume: "/root".to_string(),
        }
    }
}

fn stakgraph(
    img: &StakgraphImage,
    neo4j: &Neo4jImage,
    boltwall: &Option<BoltwallImage>,
) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("PORT={}", img.port),
        format!(
            "NEO4J_URI=bolt://{}:{}",
            domain(&neo4j.name),
            neo4j.bolt_port
        ),
        format!("NEO4J_PASSWORD={}", neo4j.password),
    ];
    if let Ok(github_request_token) = getenv("GITHUB_REQUEST_TOKEN") {
        env.push(format!("PAT={}", github_request_token))
    }
    if let Some(boltwall) = boltwall {
        if let Some(api_token) = &boltwall.stakwork_secret {
            env.push(format!("API_TOKEN={}", api_token));
        }
    }
    if let Some(rust_log) = &img.rust_log {
        env.push(format!("RUST_LOG={}", rust_log));
        if rust_log == "debug"|| rust_log == "trace" {
            env.push("RUST_BACKTRACE=1".to_string());
        }
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
