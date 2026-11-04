//! Maple's inference proxy (`ghcr.io/mapleprivacylabs/maple-proxy`): an
//! OpenAI-compatible endpoint for the open models Maple (maple.ai) serves from
//! attested enclaves. Strut's voice turns use it for a private model
//! (`VOICE_MODEL=maple/gemma4-31b`).
//!
//! How it sits in the stack:
//!   - repo2graph's strut is its only client. It gets
//!     `MAPLE_BASE_URL=http://maple.sphinx:8080/v1` and `MAPLE_API_KEY` (from
//!     the swarm's `.env`) through a `maple` link (see `repo2graph.rs`), and
//!     sends the key on every request as the bearer. The proxy holds no key:
//!     `MAPLE_API_KEY` is never set on it.
//!   - It attests Maple's AWS Nitro enclave (`MAPLE_BACKEND_URL`, the image's
//!     default `https://enclave.trymaple.ai`) and encrypts every request to it,
//!     so it reads every prompt in plaintext. Internal only: neither
//!     host-published nor traefik-fronted, CORS off (nothing in a browser calls
//!     it, and the image turns CORS on).
//!   - `cache_root` is `MAPLE_CACHE_NAMESPACE_ROOT`, the client-side secret
//!     that keeps the provider's prompt cache across proxy restarts (32 random
//!     bytes, canonical padded base64: the proxy refuses to start on anything
//!     else, an empty value included). Generated once, when the node is
//!     created, saved with the stack, and blanked by `remove_tokens`, like the
//!     voice service's token.
//!   - The node exists only once the swarm's `.env` has `MAPLE_API_KEY`
//!     (`second_brain_imgs`, `migrate_stack`). The super writes it into a new
//!     swarm's `.env` when it has one (`MAPLE_ENV` in `src/bin/super/util.rs`).
//!   - Pinned (`MAPLE_PROXY_VERSION`) and not in `auto_update`: third-party
//!     code that holds plaintext and decides whether the enclave is genuine.
//!     Maple's exact version tags are immutable (only `latest`, `0.4`, `0`
//!     move), so the pin is the code that runs. Upgrade on purpose: bump the
//!     constant for new stacks, and `UpdateNode` a running one.
//!   - Hardened: read-only root filesystem, every capability dropped,
//!     no-new-privileges, `MEM_LIMIT`. The image runs as its own non-root
//!     user. Idle it uses a few MB.
//!   - Nothing is kept, so there is no backup. The volume is only what
//!     `host_config` always mounts.

use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// The image tag a new node runs: 0.4.5, index digest
/// sha256:68ca01b7550594022203723e32468604e00339bc1882dfd77b93b3ff807f5dfb
/// (linux/amd64 and linux/arm64).
pub const MAPLE_PROXY_VERSION: &str = "0.4.5";

/// What the container may use: the proxy streams requests through, it keeps
/// nothing.
const MEM_LIMIT: i64 = 256 * 1024 * 1024;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MapleImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    /// `MAPLE_CACHE_NAMESPACE_ROOT`. A node written without one gets one.
    #[serde(default = "new_cache_root")]
    pub cache_root: String,
}

fn new_cache_root() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::encode(bytes)
}

impl MapleImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            cache_root: new_cache_root(),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    /// What strut is handed as `MAPLE_BASE_URL`: the proxy's OpenAI-compatible
    /// root, `/v1` included.
    pub fn base_url(&self) -> String {
        format!("http://{}:{}/v1", domain(&self.name), self.port)
    }
}

/// Whether the swarm's `.env` has Maple's API key: the maple node is added to
/// a stack only then (`second_brain_imgs`, `migrate_stack`).
pub fn maple_configured() -> bool {
    getenv("MAPLE_API_KEY").is_ok()
}

#[async_trait]
impl DockerConfig for MapleImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(maple(self))
    }
}

impl DockerHubImage for MapleImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::Ghcr,
            org: "mapleprivacylabs".to_string(),
            repo: "maple-proxy".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn maple(node: &MapleImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();
    let root_vol = &repo.root_volume;

    // Never MAPLE_API_KEY: each request brings its own.
    let env = vec![
        // The binary binds loopback unless told otherwise.
        "MAPLE_HOST=0.0.0.0".to_string(),
        format!("MAPLE_PORT={}", node.port),
        "MAPLE_ENABLE_CORS=false".to_string(),
        format!("MAPLE_CACHE_NAMESPACE_ROOT={}", node.cache_root),
    ];

    // No host port (the empty `ports`) and no traefik labels: strut reaches it
    // on the swarm network, and it only connects out to Maple's enclave.
    let mut hc = host_config(&name, vec![], root_vol, None, Some(MEM_LIMIT));
    if let Some(h) = hc.as_mut() {
        h.readonly_rootfs = Some(true);
        h.cap_drop = Some(vec!["ALL".to_string()]);
        h.security_opt = Some(vec!["no-new-privileges".to_string()]);
        // `host_config` names the docker host for every container. The proxy
        // has no business there.
        h.extra_hosts = None;
    }

    Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(vec![node.port.clone()]),
        env: Some(env),
        host_config: hc,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img() -> MapleImage {
        MapleImage::new("maple", MAPLE_PROXY_VERSION, "8080")
    }

    #[test]
    fn test_maple_runs_the_pinned_image_internal_only() {
        let node = img();
        let c = maple(&node);
        assert_eq!(
            c.image.unwrap(),
            "ghcr.io/mapleprivacylabs/maple-proxy:0.4.5"
        );
        assert_eq!(c.hostname.unwrap(), "maple.sphinx");
        assert!(c.labels.is_none(), "maple must not be traefik-fronted");
        let hc = c.host_config.unwrap();
        assert!(
            hc.port_bindings.unwrap().is_empty(),
            "maple must not publish a host port"
        );
        assert_eq!(hc.network_mode.unwrap(), "sphinx-swarm");
        assert_eq!(hc.readonly_rootfs, Some(true));
        assert_eq!(hc.memory, Some(MEM_LIMIT));

        let env = c.env.unwrap();
        for e in [
            "MAPLE_HOST=0.0.0.0",
            "MAPLE_PORT=8080",
            "MAPLE_ENABLE_CORS=false",
        ] {
            assert!(
                env.contains(&e.to_string()),
                "missing {}, got: {:?}",
                e,
                env
            );
        }
        assert!(env.contains(&format!("MAPLE_CACHE_NAMESPACE_ROOT={}", node.cache_root)));
        assert!(
            !env.iter().any(|e| e.starts_with("MAPLE_API_KEY=")),
            "the proxy holds no key"
        );
    }

    #[test]
    fn test_cache_root_is_32_bytes_of_padded_base64_made_once() {
        let a = img();
        assert_eq!(base64::decode(&a.cache_root).unwrap().len(), 32);
        assert_eq!(a.cache_root.len(), 44, "canonical padded base64");
        assert_ne!(a.cache_root, img().cache_root, "every node gets its own");

        let json = serde_json::to_string(&a).unwrap();
        let back: MapleImage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cache_root, a.cache_root, "a restart keeps it");

        let bare = r#"{"name":"maple","version":"0.4.5","port":"8080","links":[]}"#;
        let node: MapleImage = serde_json::from_str(bare).unwrap();
        assert_eq!(base64::decode(&node.cache_root).unwrap().len(), 32);
    }

    #[test]
    fn test_base_url_includes_v1() {
        assert_eq!(img().base_url(), "http://maple.sphinx:8080/v1");
    }
}
