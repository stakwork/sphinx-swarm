use super::{neo4j::Neo4jImage, *};
use crate::utils::{domain, exposed_ports, host_config};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct JarvisImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
}

impl JarvisImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
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

pub fn jarvis(node: &JarvisImage, neo4j: &Neo4jImage) -> Config<String> {
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
        format!("STAKWORK_ADD_EPISODE_URL=https://jobs.stakwork.com/api/v1/projects")
    ];
    // from the stack-prod.yml
    if let Ok(stakwork_key) = std::env::var("STAKWORK_ADD_NODE_TOKEN") {
        env.push(format!("STAKWORK_ADD_NODE_TOKEN={}", stakwork_key));
    }
    if let Ok(aws_key_id) = std::env::var("AWS_ACCESS_KEY_ID") {
        env.push(format!("AWS_ACCESS_KEY_ID={}", aws_key_id));
    }
    if let Ok(aws_secret) = std::env::var("AWS_SECRET_ACCESS_KEY") {
        env.push(format!("AWS_SECRET_ACCESS_KEY={}", aws_secret));
    }
    if let Ok(aws_region) = std::env::var("AWS_REGION") {
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
