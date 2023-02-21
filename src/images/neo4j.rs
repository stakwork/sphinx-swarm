use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Neo4jImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub bolt_port: String,
    pub links: Links,
}

impl Neo4jImage {
    pub fn new(name: &str, version: &str) -> Self {
        // ports are hardcoded
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "7474".to_string(),
            bolt_port: "7687".to_string(),
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for Neo4jImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(neo4j(self))
    }
}

impl DockerHubImage for Neo4jImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "".to_string(),
            repo: "neo4j".to_string(),
        }
    }
}

pub fn neo4j(node: &Neo4jImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}", repo.repo);
    let root_vol = "/data";
    let ports = vec![node.http_port.clone(), node.bolt_port.clone()];

    Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None),
        env: Some(vec![
            format!("NEO4J_AUTH=neo4j/test"),
            format!("NEO4J_apoc_export_file_enabled=true"),
            format!("NEO4J_apoc_import_file_enabled=true"),
            format!("NEO4J_dbms_security_procedures_unrestricted=apoc.*,algo.*"),
            format!("NEO4J_dbms_memory_heap_initial__size=512m"),
            format!("NEO4J_dbms_memory_heap_max__size=2G"),
            format!("NEO4J_apoc_uuid_enabled=true"),
            format!("NEO4J_dbms_default__listen__address=0.0.0.0"),
            format!("NEO4J_dbms_connector_bolt_listen__address=0.0.0.0:7687"),
            format!("NEO4J_dbms_allow__upgrade=true"),
            format!("NEO4J_dbms_default__database=neo4j"),
        ]),
        ..Default::default()
    }
}
