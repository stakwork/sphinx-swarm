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
    /// When set, Vector runs in forwarder mode: reads Docker container logs
    /// and ships them over HTTPS to a remote Vector instance at this URL.
    /// Example: "https://vector.example.com"
    pub forward_url: Option<String>,
    /// Auth token for the remote Vector instance (will be hashed with sha256_hex_24)
    pub forward_token: Option<String>,
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
            forward_url: None,
            forward_token: None,
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
    pub fn set_forward(&mut self, url: &str, token: &str) {
        self.forward_url = Some(url.to_string());
        self.forward_token = Some(token.to_string());
    }
    pub async fn pre_startup(&self, docker: &Docker, nodes: &Vec<Node>) -> Result<()> {
        let config = if let (Some(url), Some(token)) = (&self.forward_url, &self.forward_token) {
            // Forwarder mode: read Docker logs and ship to remote Vector
            let auth_token = secrets::sha256_hex_24(token);
            log::info!("=> vector forwarder mode -> {}", url);
            vector_forwarder_toml(url, &auth_token)
        } else {
            // Receiver mode: accept logs via HTTP and send to Quickwit
            let li = LinkedImages::from_nodes(self.links.clone(), nodes);
            let quickwit = li.find_quickwit();

            let quickwit_host = if let Some(qw) = quickwit {
                domain(&qw.name)
            } else {
                "quickwit.sphinx".to_string()
            };

            let base_token = if let Some(boltwall) = li.find_boltwall() {
                boltwall.stakwork_secret.unwrap_or(self.auth_token.clone())
            } else {
                self.auth_token.clone()
            };

            let auth_token = secrets::sha256_hex_24(&base_token);
            log::info!("=> vector auth token (hashed): {}", auth_token);
            vector_toml(&self.http_port, &quickwit_host, &auth_token)
        };

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

    // In forwarder mode, mount the Docker socket so Vector can read container logs
    let extra_vols = if node.forward_url.is_some() {
        Some(vec!["/var/run/docker.sock:/var/run/docker.sock:ro".to_string()])
    } else {
        None
    };

    let mut c = Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, extra_vols, None),
        // Explicitly use only our config file, no default/demo sources
        cmd: Some(vec![
            "--config".to_string(),
            "/etc/vector/vector.toml".to_string(),
        ]),
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
# SOURCE - Single HTTP server, routes by path
# =============================================================================

# HTTP server for all log ingestion
# Endpoints:
#   POST /vercel - Vercel log drain (NDJSON)
#   POST /logs   - Generic JSON logs
# Requires Authorization: Bearer <token> header
[sources.http_logs]
type = "http_server"
address = "0.0.0.0:{http_port}"
decoding.codec = "json"
framing.method = "newline_delimited"
headers = ["Authorization"]
strict_path = false
path_key = "_request_path"

# =============================================================================
# TRANSFORMS - Auth, routing, and normalization
# =============================================================================

# Check auth and route by path
[transforms.auth_and_route]
type = "remap"
inputs = ["http_logs"]
source = '''
expected = "Bearer {auth_token}"
auth_header = .Authorization
if is_null(auth_header) || auth_header != expected {{
  abort
}}

# Get the request path (stored by http_server source via path_key)
request_path = string!(._request_path)

# Route and tag based on path
if starts_with(request_path, "/vercel") {{
  .log_source = "vercel"
}} else if starts_with(request_path, "/logs") {{
  .log_source = "generic"
  # Add timestamp if missing (unix ms)
  if !exists(.timestamp) {{
    .timestamp = to_unix_timestamp(now(), unit: "milliseconds")
  }}
  # Default level if missing
  if !exists(.level) {{
    .level = "info"
  }}
}} else {{
  # Unknown path, drop it
  abort
}}

# Clean up internal fields before sending to Quickwit
del(.Authorization)
del(._request_path)
del(.source_type)
'''

# =============================================================================
# TRANSFORMS - Aggregate logs by requestId
# =============================================================================

# Aggregate all logs from the same request into a single event
# Waits up to 30 seconds for all logs from a request to arrive
[transforms.aggregate_by_request]
type = "reduce"
inputs = ["auth_and_route"]
group_by = ["requestId"]
expire_after_ms = 30000
starts_when = 'exists(.requestId)'
merge_strategies.message = "concat_newline"
merge_strategies.timestamp = "retain"
merge_strategies.level = "retain"
merge_strategies.path = "retain"
merge_strategies.host = "retain"
merge_strategies.source = "retain"
merge_strategies.environment = "retain"
merge_strategies.projectId = "retain"
merge_strategies.projectName = "retain"
merge_strategies.deploymentId = "retain"
merge_strategies.requestId = "retain"
merge_strategies.log_source = "retain"
merge_strategies.proxy = "retain"
merge_strategies.invocationId = "retain"
merge_strategies.executionRegion = "retain"
merge_strategies.id = "discard"
merge_strategies.type = "discard"

# =============================================================================
# SINKS - Send to Quickwit
# =============================================================================

[sinks.quickwit]
type = "http"
inputs = ["aggregate_by_request"]
uri = "http://{quickwit_host}:7280/api/v1/logs/ingest"
encoding.codec = "json"
framing.method = "newline_delimited"
"#
    )
}

fn vector_forwarder_toml(forward_url: &str, auth_token: &str) -> String {
    format!(
        r#"# Vector forwarder configuration
# Reads Docker container logs and ships them to a remote Vector instance

# =============================================================================
# SOURCE - Docker container logs
# =============================================================================

[sources.docker]
type = "docker_logs"
# Reads from all running containers via the Docker daemon socket
# Excludes the vector container itself to avoid log loops
exclude_containers = ["vector*"]

# =============================================================================
# TRANSFORM - Normalize Docker logs for the remote Vector /logs endpoint
# =============================================================================

[transforms.normalize]
type = "remap"
inputs = ["docker"]
source = '''
.log_source = "docker"
.timestamp = to_unix_timestamp(now(), unit: "milliseconds")
.level = if string(.stream) ?? "stdout" == "stderr" {{ "error" }} else {{ "info" }}
.container_name = string(.label."com.docker.compose.service") ?? string(.container_name) ?? ""
.container_id = string(.container_id) ?? ""

del(.stream)
del(.source_type)
del(.label)
del(.image)
del(.container_created_at)
'''

# =============================================================================
# SINK - Forward to remote Vector over HTTPS
# =============================================================================

[sinks.remote_vector]
type = "http"
inputs = ["normalize"]
uri = "{forward_url}/logs"
encoding.codec = "json"
framing.method = "newline_delimited"
[sinks.remote_vector.request]
headers.Authorization = "Bearer {auth_token}"
"#
    )
}
