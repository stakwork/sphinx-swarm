//! The treasury: the Lightning wallet strut sells paid public runs through
//! (strut `plans/paid-runs.md` §9, §11; `specs/LIGHTNING.md`). It is a Sphinx
//! daemon (`ghcr.io/stakwork/sphinx-daemon`, stakwork/sphinx `wasm/daemon`) on
//! an account of its own, apart from the swarm's `bot`, which is boltwall's
//! wallet.
//!
//! How it sits in the stack:
//!   - repo2graph's strut (the lab, at `/lab`) is its only caller, through a
//!     `treasury` link (see `repo2graph.rs`): `LIGHTNING_DAEMON_URL`, the
//!     RECEIVE token as `LIGHTNING_DAEMON_TOKEN` (invoices, lookups, payments
//!     in, the balance) and the PAY token as `LIGHTNING_DAEMON_PAY_TOKEN` (a
//!     withdrawal). Strut writes them into its own encrypted `payments.json` at
//!     boot. The ADMIN token (posting, tribes) is handed to nobody: it is kept
//!     with the stack for an operator. The three are generated once, when the
//!     node is created, and blanked by `remove_tokens`; the daemon refuses to
//!     start when any two are equal, and so does strut for the two it gets.
//!   - `DAEMON_REPLY=none` and no `DAEMON_SERVICE_URL`: it answers nobody and
//!     joins no tribe. It only connects out to its LSP (the image's default,
//!     `broker.v2.sphinx.chat`).
//!   - Internal only: neither host-published nor traefik-fronted. The admin
//!     API listens on all of the container's interfaces, so on the swarm
//!     network, where only its tokens guard it.
//!   - `/data` IS THE MONEY. The account's seed is made there on the first
//!     start (never passed in, unlike the bot's), with `wallet.jsonl`, the
//!     payments it took. So the node is in `backup_services`, and losing the
//!     volume loses the sats. One container per account: a second on the
//!     same volume would knock the first off the broker.
//!   - Part of every stack that has a repo2graph, new or existing:
//!     `second_brain_imgs` adds it to a new stack, and `migrate_stack` to an
//!     existing one on its next start.
//!   - On `latest` and in `auto_update`: strut's payer and the daemon's wallet
//!     routes move together. A recreate keeps the volume.

use super::*;
use crate::config::Node;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

/// The account's alias, set when the account is made.
const ALIAS: &str = "treasury";

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TreasuryImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
    /// `DAEMON_ADMIN_TOKEN`: every route. Nobody in the stack holds it.
    #[serde(default = "new_token")]
    pub admin_token: String,
    /// `DAEMON_RECEIVE_TOKEN`: invoices, lookups, payments in, the balance.
    /// Strut's `LIGHTNING_DAEMON_TOKEN`.
    #[serde(default = "new_token")]
    pub receive_token: String,
    /// `DAEMON_PAY_TOKEN`: the receive routes and `POST /pay`. Strut's
    /// `LIGHTNING_DAEMON_PAY_TOKEN`, kept for its payer alone.
    #[serde(default = "new_token")]
    pub pay_token: String,
}

fn new_token() -> String {
    secrets::hex_secret_32()
}

impl TreasuryImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            admin_token: new_token(),
            receive_token: new_token(),
            pay_token: new_token(),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    /// What strut is handed as `LIGHTNING_DAEMON_URL`.
    pub fn url(&self) -> String {
        format!("http://{}:{}", domain(&self.name), self.port)
    }
}

#[async_trait]
impl DockerConfig for TreasuryImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(treasury(self))
    }
}

impl DockerHubImage for TreasuryImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::Ghcr,
            org: "stakwork".to_string(),
            repo: "sphinx-daemon".to_string(),
            // The account folder: the seed and its sats (see the module doc).
            root_volume: "/data".to_string(),
        }
    }
}

fn treasury(node: &TreasuryImage) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = node.image();
    let root_vol = &repo.root_volume;

    let env = vec![
        format!("DAEMON_ADMIN_TOKEN={}", node.admin_token),
        format!("DAEMON_RECEIVE_TOKEN={}", node.receive_token),
        format!("DAEMON_PAY_TOKEN={}", node.pay_token),
        format!("DAEMON_ALIAS={}", ALIAS),
        // No service URL: it answers nobody and joins no tribe.
        "DAEMON_REPLY=none".to_string(),
        // The image's own defaults, said here because the seed must land on
        // the volume this config mounts, and strut must reach the port.
        format!("SPHINX_HOME={}", root_vol),
        "DAEMON_HOST=0.0.0.0".to_string(),
        format!("DAEMON_PORT={}", node.port),
    ];

    // No host port (the empty `ports`) and no traefik labels: strut reaches it
    // on the swarm network, and it only connects out to its LSP.
    let mut hc = host_config(&name, vec![], root_vol, None, None);
    if let Some(h) = hc.as_mut() {
        // The image runs as its own `node` user and needs no capability.
        h.cap_drop = Some(vec!["ALL".to_string()]);
        h.security_opt = Some(vec!["no-new-privileges".to_string()]);
        // `host_config` names the docker host for every container. The
        // treasury has no business there.
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

    fn img() -> TreasuryImage {
        TreasuryImage::new("treasury", "latest", "8095")
    }

    #[test]
    fn test_treasury_runs_the_daemon_image_internal_only() {
        let c = treasury(&img());
        assert_eq!(c.image.unwrap(), "ghcr.io/stakwork/sphinx-daemon:latest");
        assert_eq!(c.hostname.unwrap(), "treasury.sphinx");
        assert!(
            c.labels.is_none(),
            "the treasury must not be traefik-fronted"
        );
        let hc = c.host_config.unwrap();
        assert!(
            hc.port_bindings.unwrap().is_empty(),
            "the treasury must not publish a host port"
        );
        assert_eq!(hc.network_mode.unwrap(), "sphinx-swarm");
        assert_eq!(hc.cap_drop, Some(vec!["ALL".to_string()]));
        assert_eq!(hc.security_opt, Some(vec!["no-new-privileges".to_string()]));
    }

    #[test]
    fn test_the_seed_lands_on_the_volume() {
        let c = treasury(&img());
        let binds = c.host_config.unwrap().binds.unwrap();
        assert!(
            binds.contains(&"treasury.sphinx:/data:rw".to_string()),
            "got: {:?}",
            binds
        );
        assert!(c.env.unwrap().contains(&"SPHINX_HOME=/data".to_string()));
    }

    #[test]
    fn test_treasury_env_names_its_tokens_and_joins_nothing() {
        let node = img();
        let env = treasury(&node).env.unwrap();
        for e in [
            "DAEMON_ALIAS=treasury",
            "DAEMON_REPLY=none",
            "DAEMON_HOST=0.0.0.0",
            "DAEMON_PORT=8095",
        ] {
            assert!(
                env.contains(&e.to_string()),
                "missing {}, got: {:?}",
                e,
                env
            );
        }
        assert!(env.contains(&format!("DAEMON_ADMIN_TOKEN={}", node.admin_token)));
        assert!(env.contains(&format!("DAEMON_RECEIVE_TOKEN={}", node.receive_token)));
        assert!(env.contains(&format!("DAEMON_PAY_TOKEN={}", node.pay_token)));
        assert!(
            !env.iter().any(|e| e.starts_with("DAEMON_SERVICE_URL=")),
            "no service: it joins no tribe"
        );
    }

    #[test]
    fn test_the_three_tokens_are_secrets_made_once_and_differ() {
        let a = img();
        for t in [&a.admin_token, &a.receive_token, &a.pay_token] {
            assert_eq!(t.len(), 64);
            assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
        }
        // the daemon refuses to start on two equal tokens
        assert_ne!(a.admin_token, a.receive_token);
        assert_ne!(a.admin_token, a.pay_token);
        assert_ne!(a.receive_token, a.pay_token);
        assert_ne!(
            a.receive_token,
            img().receive_token,
            "every node gets its own"
        );

        // persisted with the stack, so a restart keeps them
        let json = serde_json::to_string(&a).unwrap();
        let back: TreasuryImage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn test_a_node_written_without_tokens_gets_three_different_ones() {
        let json = r#"{"name":"treasury","version":"latest","port":"8095","links":[]}"#;
        let node: TreasuryImage = serde_json::from_str(json).unwrap();
        assert_eq!(node.receive_token.len(), 64);
        assert_ne!(node.admin_token, node.receive_token);
        assert_ne!(node.admin_token, node.pay_token);
        assert_ne!(node.receive_token, node.pay_token);
    }

    #[test]
    fn test_url_has_no_secret_in_it() {
        assert_eq!(img().url(), "http://treasury.sphinx:8095");
    }
}
