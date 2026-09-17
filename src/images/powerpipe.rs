//! Steampipe (AWS plugin) + Powerpipe (aws-thrifty) in a single third-party
//! container. Auth is **instance-profile only** — the host EC2 role already
//! attached by `create_ec2_instance` (`AWS_USER_ROLE` + IMDSv2 hop-limit 2).
//! Static `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` are never set or copied.
//!
//! The `sphinxlightning/powerpipe` image (Dockerfile, `start.sh` with IMDSv2
//! probe/retry and Steampipe readiness) is published outside this repo, same
//! as `sphinxlightning/rqbit` and `sphinxlightning/dufs`.
//!
//! Isolation follows Hermes, not rqbit: no Traefik labels, no host port. An
//! unauthenticated Powerpipe API on a published/fronted port could query/spend
//! the instance credential. Reachable only on the sphinx-swarm network at
//! `http://{name}.sphinx:{port}` (default `powerpipe.sphinx:9033`).
//!
//! Opt-in only — not in `second_brain_imgs` or `migrate_stack`.

use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct PowerpipeImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub links: Links,
}

impl PowerpipeImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
}

#[async_trait]
impl DockerConfig for PowerpipeImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        powerpipe(self)
    }
}

impl DockerHubImage for PowerpipeImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "powerpipe".to_string(),
            root_volume: "/opt/data".to_string(),
        }
    }
}

/// Build the container Config.
///
/// Threat model (Hermes-style): this container holds the host's instance-profile
/// AWS credentials internally; it must never be Traefik-fronted or have a host
/// port published, or anything reachable on that port could query/spend the AWS
/// role. Keep it on the sphinx-swarm network. IMDS probe/retry and Steampipe
/// readiness live in the externally-published image's `start.sh`, not here.
fn powerpipe(img: &PowerpipeImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();
    let root_vol = &repo.root_volume;

    let mut env = vec![
        format!("PORT={}", img.port),
        "STEAMPIPE_INSTALL_DIR=/opt/steampipe".to_string(),
    ];
    // Region is not a credential; forward it when the host has one so the AWS
    // plugin doesn't have to guess. Never copy access keys.
    if let Ok(region) = getenv("AWS_REGION") {
        env.push(format!("AWS_REGION={}", region));
    }

    Ok(Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(vec![img.port.clone()]),
        host_config: host_config(&img.name, vec![], root_vol, None, None),
        env: Some(env),
        entrypoint: Some(vec!["/usr/local/bin/start.sh".to_string()]),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn restore_env(key: &str, prev: Option<String>) {
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn test_powerpipe_host_is_none() {
        let img = Image::Powerpipe(PowerpipeImage::new("powerpipe", "latest", "9033"));
        assert_eq!(img.host(), None);
    }

    #[test]
    fn test_powerpipe_port_is_not_published_to_the_host() {
        let img = PowerpipeImage::new("powerpipe", "latest", "9033");
        let c = powerpipe(&img).unwrap();

        let bindings = c.host_config.unwrap().port_bindings.unwrap();
        assert!(
            bindings.is_empty(),
            "powerpipe must not publish a host port, got: {:?}",
            bindings
        );
    }

    #[test]
    fn test_powerpipe_has_no_traefik_labels() {
        let img = PowerpipeImage::new("powerpipe", "latest", "9033");
        let c = powerpipe(&img).unwrap();
        assert!(
            c.labels.is_none(),
            "powerpipe must never be Traefik-fronted, got: {:?}",
            c.labels
        );
    }

    #[test]
    fn test_powerpipe_env_never_contains_static_aws_keys() {
        let _lock = ENV_LOCK.lock().unwrap();

        let prev_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let prev_secret = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
        std::env::set_var("AWS_ACCESS_KEY_ID", "AKIA_DUMMY_TEST_KEY");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "dummy_secret_value");

        let img = PowerpipeImage::new("powerpipe", "latest", "9033");
        let env = powerpipe(&img).unwrap().env.unwrap();

        restore_env("AWS_ACCESS_KEY_ID", prev_id);
        restore_env("AWS_SECRET_ACCESS_KEY", prev_secret);

        assert!(
            env.iter().any(|e| e.starts_with("PORT=")),
            "env should be populated (PORT=...), got: {:?}",
            env
        );
        assert!(
            !env.iter().any(|e| e.starts_with("AWS_ACCESS_KEY_ID=")),
            "must not copy AWS_ACCESS_KEY_ID, got: {:?}",
            env
        );
        assert!(
            !env.iter().any(|e| e.starts_with("AWS_SECRET_ACCESS_KEY=")),
            "must not copy AWS_SECRET_ACCESS_KEY, got: {:?}",
            env
        );
    }

    #[test]
    fn test_powerpipe_volume_is_opt_data() {
        let img = PowerpipeImage::new("powerpipe", "latest", "9033");
        let c = powerpipe(&img).unwrap();

        let binds = c.host_config.unwrap().binds.unwrap();
        assert!(
            binds.iter().any(|b| b == "powerpipe.sphinx:/opt/data:rw"),
            "binds should mount the named volume at /opt/data, got: {:?}",
            binds
        );
        assert!(
            !binds.iter().any(|b| b.contains("/var/opt/steampipe")
                || b.contains(".steampipe")
                || b.contains("/home")),
            "must not bind a Steampipe/plugin path, got: {:?}",
            binds
        );
    }

    #[test]
    fn test_powerpipe_entrypoint_is_baked_in_start_sh() {
        let img = PowerpipeImage::new("powerpipe", "latest", "9033");
        let c = powerpipe(&img).unwrap();

        assert_eq!(
            c.entrypoint.as_ref().unwrap(),
            &vec!["/usr/local/bin/start.sh".to_string()]
        );
        // Cmd must not carry an inline IMDS curl/retry script; that lives in
        // the published image's start.sh.
        if let Some(cmd) = &c.cmd {
            assert_eq!(cmd, &vec!["/usr/local/bin/start.sh".to_string()]);
            assert!(
                !cmd.iter().any(|s| s.contains('\n') || s.contains("curl")),
                "cmd must not be an inline IMDS script, got: {:?}",
                cmd
            );
        }
    }
}
