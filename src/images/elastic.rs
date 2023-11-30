// use super::traefik::{neo4j_labels, traefik_labels};
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

    pub async fn post_startup(&self, proj: &str, docker: &Docker) -> Result<()> {
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
            org: "library".to_string(),
            repo: "elasticsearch".to_string(),
        }
    }
}

fn elastic(node: &ElasticImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}", repo.repo);
    let root_vol = "/data";
    let ports = vec![node.http_port.clone()];

    let c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None),
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

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}

const APOC_CONF: &str = r#"
apoc.import.file.use_neo4j_config=true
apoc.import.file.enabled=true
apoc.export.file.enabled=true
apoc.initializer.neo4j.1=CREATE FULLTEXT INDEX titles_full_index IF NOT EXISTS FOR (n:Content) ON EACH [n.show_title, n.episode_title]
apoc.initializer.neo4j.2=CREATE FULLTEXT INDEX person_full_index IF NOT EXISTS FOR (n:Person) ON EACH [n.name]
apoc.initializer.neo4j.3=CREATE FULLTEXT INDEX topic_full_index IF NOT EXISTS FOR (n:Topic) ON EACH [n.topic]
apoc.initializer.neo4j.4=CREATE FULLTEXT INDEX text_full_index IF NOT EXISTS FOR (n:Content) ON EACH [n.text]
apoc.initializer.neo4j.5=CREATE FULLTEXT INDEX data_bank_full_index IF NOT EXISTS FOR (n:Data_Bank) ON EACH [n.Data_Bank]
apoc.initializer.neo4j.6=MATCH (n) WHERE NOT EXISTS(n.namespace) OR n.namespace = '' SET n.namespace = 'default'
"#;
