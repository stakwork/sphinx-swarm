//! The browser that repo2graph's workflow steps drive: a Playwright SERVER
//! (`playwright run-server`), built from `images/browser` in stakwork/strut
//! and published as `ghcr.io/stakwork/strut-browser`.
//!
//! How it sits in the stack:
//!   - repo2graph is its only client. It connects to
//!     `ws://browser.sphinx:3000/<ws_path>` (`BROWSER_WS_URL` and
//!     `BROWSER_WS_PATH`, see `repo2graph.rs`). The server launches one
//!     headless Chromium for that connection, and repo2graph opens a browser
//!     context per workflow run inside it.
//!   - `ws_path` is the secret endpoint, the browser's only door: the server
//!     refuses a connection on any other path. It is generated once, when the
//!     node is created, and persisted with the stack.
//!   - A network of its own (`dock::BROWSER_NETWORK`), which repo2graph joins
//!     as a second network. A page can reach whatever the browser's network
//!     reaches, so the browser is kept off `sphinx-swarm`, where neo4j, redis
//!     and the rest answer by name.
//!   - Neither host-published nor traefik-fronted.
//!   - On `latest` and in `auto_update`, like repo2graph. The server refuses a
//!     client whose Playwright major.minor differs from its own, so the image
//!     and repo2graph's `playwright-core` are released together and both are
//!     picked up by the same daily pass.

use super::*;
use crate::config::Node;
use crate::dock::BROWSER_NETWORK;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BrowserImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    /// The secret websocket path. A node written without one gets one.
    #[serde(default = "new_ws_path")]
    pub ws_path: String,
}

fn new_ws_path() -> String {
    secrets::hex_secret_32()
}

impl BrowserImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            ws_path: new_ws_path(),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    /// What a client connects to, without the secret path.
    pub fn ws_url(&self) -> String {
        format!("ws://{}:{}", domain(&self.name), self.port)
    }
}

#[async_trait]
impl DockerConfig for BrowserImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(browser(self))
    }
}

impl DockerHubImage for BrowserImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::Ghcr,
            org: "stakwork".to_string(),
            repo: "strut-browser".to_string(),
            // Nothing is kept: a run's browser context is closed when the run
            // ends. The volume is only what `host_config` always mounts.
            root_volume: "/data".to_string(),
        }
    }
}

fn browser(node: &BrowserImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();

    let root_vol = &repo.root_volume;
    let ports = vec![node.port.clone()];

    let env = vec![
        format!("PORT={}", node.port),
        format!("BROWSER_WS_PATH={}", node.ws_path),
    ];

    // No host port (the empty `ports`) and no traefik labels: the websocket is
    // reachable from repo2graph only, over the browser's own network.
    let mut hc = host_config(&name, vec![], root_vol, None, None);
    if let Some(h) = hc.as_mut() {
        h.network_mode = Some(BROWSER_NETWORK.to_string());
        // `host_config` names the docker host `host.docker.internal` for every
        // container. A page has no business there.
        h.extra_hosts = None;
    }

    Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports),
        env: Some(env),
        host_config: hc,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img() -> BrowserImage {
        BrowserImage::new("browser", "latest", "3000")
    }

    #[test]
    fn test_browser_runs_the_published_image_behind_its_secret_path() {
        let node = img();
        let c = browser(&node);

        assert_eq!(c.image.unwrap(), "ghcr.io/stakwork/strut-browser:latest");
        assert_eq!(c.hostname.unwrap(), "browser.sphinx");

        let env = c.env.unwrap();
        assert!(env.contains(&"PORT=3000".to_string()), "got: {:?}", env);
        assert!(
            env.contains(&format!("BROWSER_WS_PATH={}", node.ws_path)),
            "got: {:?}",
            env
        );
        // the image's own command runs the server
        assert!(c.cmd.is_none());
    }

    #[test]
    fn test_browser_is_on_its_own_network_only() {
        let c = browser(&img());
        let hc = c.host_config.unwrap();

        assert_eq!(hc.network_mode.unwrap(), "sphinx-browser");
        assert!(c.networking_config.is_none());
        assert!(
            hc.extra_hosts.is_none(),
            "the browser must not get a name for the docker host"
        );
    }

    #[test]
    fn test_browser_port_is_not_published_to_the_host() {
        let c = browser(&img());

        // Anything that can reach the port and knows the path drives the
        // browser, so it is not put on a host interface.
        let bindings = c.host_config.unwrap().port_bindings.unwrap();
        assert!(
            bindings.is_empty(),
            "browser must not publish a host port, got: {:?}",
            bindings
        );
        assert!(c.labels.is_none(), "browser must not be traefik-fronted");
    }

    #[test]
    fn test_ws_path_is_a_secret_made_once() {
        let a = img();
        let b = img();
        assert_eq!(a.ws_path.len(), 64);
        assert!(a.ws_path.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a.ws_path, b.ws_path, "every node gets its own path");

        // persisted with the stack, so a restart keeps it
        let json = serde_json::to_string(&a).unwrap();
        let back: BrowserImage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ws_path, a.ws_path);
    }

    #[test]
    fn test_a_node_written_without_a_path_gets_one() {
        let json = r#"{"name":"browser","version":"latest","port":"3000","links":[]}"#;
        let node: BrowserImage = serde_json::from_str(json).unwrap();
        assert_eq!(node.ws_path.len(), 64);
    }

    #[test]
    fn test_ws_url_has_no_secret_in_it() {
        let node = img();
        assert_eq!(node.ws_url(), "ws://browser.sphinx:3000");
    }
}
