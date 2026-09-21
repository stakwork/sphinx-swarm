use super::*;
use crate::config::Node;
use crate::images::neo4j::Neo4jImage;
use crate::images::repo2graph::Repo2GraphImage;
use crate::images::boltwall::BoltwallImage;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

/// aws-advisor: the AWS cost advisor (Steampipe + Powerpipe + rules + the repo2graph agent + a React UI),
/// added to the graph-mindset stack when DEVOPS=1. Read-only towards AWS by design: the container gets no
/// AWS credentials from the swarm; they are entered in its Settings page (or come from the host's instance role).
///
/// Private, like neo4j: no Traefik route and no public hostname. The UI lists the account's instances, probes,
/// costs and decisions, so it is reachable only on the host's private IP (port 9034), the way the Neo4j browser is.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct AdvisorImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    pub host: Option<String>,
    /// gates the API and the UI; generated on first boot when absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,
    /// gates the /mcp fact server the agent calls back into
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_token: Option<String>,
    /// echoed by repo2graph on the webhook
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_secret: Option<String>,
}

impl AdvisorImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            host: None,
            api_token: Some(secrets::random_word(32)),
            mcp_token: Some(secrets::random_word(32)),
            callback_secret: Some(secrets::random_word(32)),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    /// Kept for the Image API; the advisor never gets a public host (see the type doc).
    pub fn host(&mut self, _eh: Option<String>) {
        self.host = None;
    }
}

#[async_trait]
impl DockerConfig for AdvisorImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        Ok(advisor(self, &li.find_repo2graph(), &li.find_boltwall(), &li.find_neo4j()))
    }
}

impl DockerHubImage for AdvisorImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::Ghcr,
            org: "stakwork".to_string(),
            repo: "aws-advisor".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn advisor(
    img: &AdvisorImage,
    repo2graph: &Option<Repo2GraphImage>,
    boltwall: &Option<BoltwallImage>,
    neo4j: &Option<Neo4jImage>,
) -> Config<String> {
    let repo = img.repo();
    let image = img.image();
    let root_vol = &repo.root_volume;
    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("PORT={}", img.port),
        // the address repo2graph's container uses for the webhook and the /mcp fact server
        format!("PUBLIC_URL=http://{}:{}", domain(&img.name), img.port),
    ];
    if let Some(t) = &img.api_token {
        env.push(format!("API_TOKEN={}", t));
    }
    if let Some(t) = &img.mcp_token {
        env.push(format!("MCP_TOKEN={}", t));
    }
    if let Some(t) = &img.callback_secret {
        env.push(format!("CALLBACK_SECRET={}", t));
    }
    if let Some(r2g) = repo2graph {
        env.push(format!("REPO2GRAPH_URL=http://{}:{}", domain(&r2g.name), r2g.port));
        if let Some(b) = boltwall {
            if let Some(secret) = &b.stakwork_secret {
                env.push(format!("REPO2GRAPH_TOKEN={}", secret));
            }
        }
    }
    if let Some(n) = neo4j {
        env.push(format!("NEO4J_URI=bolt://{}:{}", domain(&n.name), n.bolt_port));
        env.push("NEO4J_USER=neo4j".to_string());
        env.push(format!("NEO4J_PASSWORD={}", n.password));
    }
    // Nothing else: the agent model and key, the TypeSafe key, every schedule and threshold are set on the
    // advisor's Settings page and stored in its own database. Without a key of its own the advisor's agent
    // runs on repo2graph's key.

    // no Traefik labels on purpose: private-IP only, like neo4j
    Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, None, None),
        env: Some(env),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn advisor_gets_its_three_secrets_and_the_agent_endpoints() {
        let _lock = ENV_LOCK.lock().unwrap();
        let img = AdvisorImage::new("advisor", "latest", "9034");
        let mut r2g = Repo2GraphImage::new("repo2graph", "latest", "3355");
        r2g.links(vec!["neo4j"]);
        let neo4j = Neo4jImage::new("neo4j", "5.19.0");
        let c = advisor(&img, &Some(r2g), &None, &Some(neo4j));
        let env = c.env.unwrap();
        assert!(env.iter().any(|e| e.starts_with("API_TOKEN=") && e.len() > 20));
        assert!(env.iter().any(|e| e.starts_with("MCP_TOKEN=")));
        assert!(env.iter().any(|e| e.starts_with("CALLBACK_SECRET=")));
        assert!(env.contains(&"REPO2GRAPH_URL=http://repo2graph.sphinx:3355".to_string()));
        assert!(env.contains(&"PUBLIC_URL=http://advisor.sphinx:9034".to_string()));
        assert!(env.iter().any(|e| e.starts_with("NEO4J_URI=bolt://neo4j.sphinx:")));
        assert_eq!(c.image.unwrap(), "ghcr.io/stakwork/aws-advisor:latest");
        assert!(c.labels.is_none(), "no traefik route: the advisor is private, like neo4j");
    }

    #[test]
    fn advisor_never_gets_a_public_host() {
        let mut img = AdvisorImage::new("advisor", "latest", "9034");
        img.host(Some("swarm38.sphinx.chat".to_string()));
        assert!(img.host.is_none());
    }
}
