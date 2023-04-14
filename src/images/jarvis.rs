use super::{boltwall::BoltwallImage, neo4j::Neo4jImage, *};
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct JarvisImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub self_generating: bool,
    pub links: Links,
}

impl JarvisImage {
    pub fn new(name: &str, version: &str, port: &str, self_generating: bool) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            self_generating: self_generating,
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for JarvisImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let neo4j_node = li.find_neo4j().context("Jarvis: No Neo4j")?;
        let boltwall_node = li.find_boltwall().context("Jarvis: No Boltwall")?;
        Ok(jarvis(&self, &neo4j_node, &boltwall_node))
    }
}

impl DockerHubImage for JarvisImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-jarvis-backend".to_string(),
        }
    }
}

fn jarvis(node: &JarvisImage, neo4j: &Neo4jImage, boltwall: &BoltwallImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/data/jarvis";
    let ports = vec![node.port.clone()];

    let mut env = vec![
        format!(
            "NEO4J_URI=neo4j://{}.sphinx:{}",
            neo4j.name, neo4j.bolt_port
        ),
        format!("NEO4J_USER=neo4j"),
        format!("NEO4J_PASS=test"),
        format!("STAKWORK_REQUEST_LOG_PATH=./"),
        format!("STAKWORK_ADD_NODE_URL=https://api.stakwork.com/api/v1/knowledge_graph_projects"),
        format!("JARVIS_BACKEND_PORT={}", node.port),
        format!("PUBLIC_GRAPH_RESULT_LIMIT=10"),
        format!("AWS_S3_BUCKET_PATH=https://stakwork-uploads.s3.amazonaws.com/knowledge-graph-joe/content-images/"),
        format!("STAKWORK_ADD_EPISODE_URL=https://jobs.stakwork.com/api/v1/projects"),
        format!("RADAR_SCHEDULER_TIME_IN_SEC=86400"),
        format!("RADAR_REQUEST_URL=https://jobs.stakwork.com/api/v1/projects"),
        format!("RADAR_SCHEDULER_JOB=1"),
    ];
    if node.self_generating {
        env.push(format!("SELF_GENERATING_GRAPH=1"));
    }
    if let Some(h) = &boltwall.host {
        env.push(format!("RADAR_TWEET_WEBHOOK=https://{}", h));
        env.push(format!("RADAR_TOPIC_WEBHOOK=https://{}", h));
    }
    // from the stack-prod.yml
    if let Ok(stakwork_key) = std::env::var("STAKWORK_ADD_NODE_TOKEN") {
        env.push(format!("STAKWORK_ADD_NODE_TOKEN={}", stakwork_key));
    }
    if let Ok(stakwork_radar_token) = std::env::var("STAKWORK_RADAR_REQUEST_TOKEN") {
        env.push(format!("RADAR_REQUEST_TOKEN={}", stakwork_radar_token));
    }
    if let Ok(aws_key_id) = std::env::var("AWS_ACCESS_KEY_ID") {
        env.push(format!("AWS_ACCESS_KEY_ID={}", aws_key_id));
    }
    if let Ok(aws_secret) = std::env::var("AWS_SECRET_ACCESS_KEY") {
        env.push(format!("AWS_SECRET_ACCESS_KEY={}", aws_secret));
    }
    if let Ok(aws_region) = std::env::var("AWS_S3_REGION_NAME") {
        env.push(format!("AWS_S3_REGION_NAME={}", aws_region));
    }
    Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None),
        env: Some(env),
        ..Default::default()
    }
}
