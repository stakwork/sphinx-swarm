use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

// docker run --name mongodb -p 27017:27017 -d mongodb/mongodb-community-server:latest

/*

// ssh into the running container
// Change container name if necessary
$ docker exec -it mongo.sphinx /bin/bash

// Enter into mongo shell
$ mongosh

// Caret will change when you enter successfully
// Switch to admin database
$> use admin
$> db.auth("admin", passwordPrompt())

// Show available databases
$> show dbs

*/

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MongoImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub user: String,
    pub pass: String,
    pub links: Links,
    pub host: Option<String>,
}

impl MongoImage {
    pub fn new(name: &str, version: &str) -> Self {
        // ports are hardcoded
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "27017".to_string(),
            user: "root".to_string(),
            pass: "password".to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("mongo.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for MongoImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(mongo(self))
    }
}

impl DockerHubImage for MongoImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "library".to_string(),
            repo: "mongo".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn mongo(node: &MongoImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = node.image();
    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let env = Vec::new();
    // let env = vec![
    //     "MONGO_INITDB_ROOT_USERNAME=root".to_string(),
    //     "MONGO_INITDB_ROOT_PASSWORD=password".to_string(),
    // ];

    let c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None, None),
        env: Some(env),
        entrypoint: Some(vec![
            "mongod".to_string(),
            // "--auth".to_string(),
            "--bind_ip_all".to_string(),
            "--dbpath".to_string(),
            "/data/db".to_string(),
        ]),
        ..Default::default()
    };
    c
}
