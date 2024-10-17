use super::*;
use crate::config::Node;
use crate::images::mongo::MongoImage;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ChatImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub links: Links,
    pub host: Option<String>,
}

impl ChatImage {
    pub fn new(name: &str, version: &str) -> Self {
        // ports are hardcoded
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "8282".to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("chat.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for ChatImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let mongo = li.find_mongo().context("Chat: No Mongo")?;
        Ok(chat(self, &mongo))
    }
}

impl DockerHubImage for ChatImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "huggingface".to_string(),
            repo: "chat-ui".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn chat(node: &ChatImage, mongo: &MongoImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image_end = format!("{}/{}", repo.org, repo.repo);
    let image = format!("ghcr.io/{}", image_end);

    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let env = vec![format!(
        "MONGODB_URL=mongodb://{}:{}",
        domain(&mongo.name),
        mongo.http_port
    )];
    // let env = vec![format!(
    //     "MONGODB_URL=mongodb://{}:{}@{}:{}",
    //     mongo.user,
    //     mongo.pass,
    //     domain(&mongo.name),
    //     mongo.http_port
    // )];

    let c = Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        env: Some(env),
        host_config: host_config(&name, ports, root_vol, None, None),
        ..Default::default()
    };
    c
}
