use super::*;
use crate::config::Node;
use crate::dock::exec;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ElasticImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub links: Links,
    pub host: Option<String>,
}

impl ElasticImage {
    pub fn new(name: &str, version: &str) -> Self {
        // ports are hardcoded
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "9200".to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("elastic.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }

    pub async fn post_startup(&self, _proj: &str, docker: &Docker) -> Result<()> {
        let command = "/usr/share/elasticsearch/bin/elasticsearch-plugin install analysis-phonetic";

        log::info!("=> running command {}...", command);

        exec(docker, &domain(&self.name), command).await?;

        Ok(())
    }
}

#[async_trait]
impl DockerConfig for ElasticImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(elastic(self))
    }
}

impl DockerHubImage for ElasticImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "library".to_string(),
            repo: "elasticsearch".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn elastic(node: &ElasticImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = node.image();
    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None, None),
        env: Some(vec![
            format!("node.name=elastic"),
            format!("bootstrap.memory_lock=true"),
            format!("ES_JAVA_OPTS=-Xms512m -Xmx512m"),
            format!("http.cors.enabled=true"),
            format!("http.cors.allow-origin=/.*/"),
            format!("xpack.security.enabled=false"),
            format!("discovery.type=single-node"),
            format!("network.host=0.0.0.0"),
            format!("http.port={}", &node.http_port),
        ]),
        ..Default::default()
    };
    c
}
