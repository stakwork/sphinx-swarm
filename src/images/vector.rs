use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::dock::upload_to_container;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct VectorImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub links: Links,
    pub host: Option<String>,
    pub auth_token: String,
}

impl VectorImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "9000".to_string(),
            links: vec![],
            host: None,
            auth_token: secrets::random_word(32),
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("vector.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub async fn pre_startup(&self, docker: &Docker, nodes: &Vec<Node>) -> Result<()> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let quickwit = li.find_quickwit();

        let quickwit_host = if let Some(qw) = quickwit {
            domain(&qw.name)
        } else {
            // Default to quickwit.sphinx if no linked quickwit found
            "quickwit.sphinx".to_string()
        };

        // Use boltwall's stakwork_secret if linked, otherwise use our own token
        let auth_token = if let Some(boltwall) = li.find_boltwall() {
            boltwall.stakwork_secret.unwrap_or(self.auth_token.clone())
        } else {
            self.auth_token.clone()
        };

        let config = vector_toml(&self.http_port, &quickwit_host, &auth_token);

        log::info!("=> uploading vector.toml config...");
        upload_to_container(
            docker,
            &self.name,
            "/etc/vector",
            "vector.toml",
            config.as_bytes(),
        )
        .await?;

        Ok(())
    }
}

#[async_trait]
impl DockerConfig for VectorImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(vector(self))
    }
}

impl DockerHubImage for VectorImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "timberio".to_string(),
            repo: "vector".to_string(),
            root_volume: "/etc/vector".to_string(),
        }
    }
}

fn vector(node: &VectorImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let mut c = Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None, None),
        ..Default::default()
    };
    if let Some(host) = &node.host {
        c.labels = Some(traefik_labels(&node.name, &host, &node.http_port, false))
    }
    c
}

fn vector_toml(http_port: &str, quickwit_host: &str, auth_token: &str) -> String {
    format!(
        r#"# Vector configuration for log ingestion
# Receives logs via HTTP and forwards to Quickwit

# =============================================================================
# SOURCES - Multiple endpoints for different log formats
# =============================================================================

# Vercel log drain endpoint: POST /vercel
# Expects NDJSON format from Vercel
# Requires Authorization: Bearer <token> header
[sources.vercel_logs]
type = "http_server"
address = "0.0.0.0:{http_port}"
path = "/vercel"
decoding.codec = "json"
framing.method = "newline_delimited"
headers = ["Authorization"]

# Generic JSON logs endpoint: POST /logs
# Expects JSON objects (single or newline-delimited)
# Requires Authorization: Bearer <token> header
[sources.generic_logs]
type = "http_server"
address = "0.0.0.0:{http_port}"
path = "/logs"
decoding.codec = "json"
framing.method = "newline_delimited"
headers = ["Authorization"]

# =============================================================================
# TRANSFORMS - Auth check and normalize logs
# =============================================================================

# Check auth for Vercel logs
[transforms.vercel_auth]
type = "remap"
inputs = ["vercel_logs"]
source = '''
expected = "Bearer {auth_token}"
auth_header = get!(."Authorization")
if auth_header != expected {{
  abort
}}
del(."Authorization")
'''

# Check auth for generic logs
[transforms.generic_auth]
type = "remap"
inputs = ["generic_logs"]
source = '''
expected = "Bearer {auth_token}"
auth_header = get!(."Authorization")
if auth_header != expected {{
  abort
}}
del(."Authorization")
'''

# Transform Vercel logs - already has timestamp, level, message, source
[transforms.vercel_normalized]
type = "remap"
inputs = ["vercel_auth"]
source = '''
# Vercel timestamp is unix milliseconds, keep as-is for Quickwit
.log_source = "vercel"
'''

# Transform generic logs - ensure required fields exist
[transforms.generic_normalized]
type = "remap"
inputs = ["generic_auth"]
source = '''
.log_source = "generic"
# Add timestamp if missing (unix ms)
if !exists(.timestamp) {{
  .timestamp = to_unix_timestamp(now(), unit: "milliseconds")
}}
# Default level if missing
if !exists(.level) {{
  .level = "info"
}}
'''

# =============================================================================
# SINKS - Send to Quickwit
# =============================================================================

[sinks.quickwit]
type = "http"
inputs = ["vercel_normalized", "generic_normalized"]
uri = "http://{quickwit_host}:7280/api/v1/logs/ingest"
encoding.codec = "json"
framing.method = "newline_delimited"
"#
    )
}
