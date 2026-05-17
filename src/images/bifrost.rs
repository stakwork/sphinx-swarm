use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::boltwall::BoltwallImage;
use crate::images::redis::RedisImage;
use crate::secrets;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BifrostImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
    // `serde(default = ...)` keeps existing persisted swarm state
    // loadable: pre-Phase-3 stacks have no `admin_user` / `admin_password`
    // in their stored JSON. On first load after upgrade they get backfilled
    // with the defaults (admin / fresh 16-char password) and re-saved.
    #[serde(default = "default_admin_user")]
    pub admin_user: String,
    #[serde(default = "default_admin_password")]
    pub admin_password: String,
}

fn default_admin_user() -> String {
    "admin".to_string()
}

fn default_admin_password() -> String {
    // 16 chars from [A-Za-z0-9] => ~95 bits of entropy.
    secrets::random_word(16)
}

impl BifrostImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            // Public port on the stakgraph-gateway container: the
            // wrapper binary (PID 1) listens here and reverse-proxies
            // to bifrost-http and the plugin admin API on internal
            // loopback ports. 8181 matches Hive's DEFAULT_BIFROST_PORT
            // and the historical port the MCP tests use.
            port: "8181".to_string(),
            links: vec![],
            host: None,
            admin_user: default_admin_user(),
            // Bifrost bcrypts this on first boot; the plaintext is also
            // exposed to Hive via /_plugin/admin-credentials so workspace
            // admins can copy-paste it into the dashboard login.
            admin_password: default_admin_password(),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("bifrost.{}", h));
        }
    }
}

#[async_trait]
impl DockerConfig for BifrostImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let boltwall = li.find_boltwall();
        // Redis is optional from a typing standpoint but required for
        // macaroon enforcement to run in anything beyond observability
        // mode (see gateway/plans/phases/phase-6-plugin-enforcement.md).
        // If a stack ships Bifrost without a redis link, the plugin
        // logs revocation/budget checks but does not enforce them.
        let redis = li.find_redis();
        Ok(bifrost(self, &boltwall, &redis))
    }
}

impl DockerHubImage for BifrostImage {
    fn repo(&self) -> Repository {
        Repository {
            // Published from stakwork/stakgraph CI to GHCR (same org as
            // repo2graph's stakgraph-mcp image). The Docker Hub mirror
            // `sphinxlightning/stakgraph-gateway` does not exist, so
            // pinning to GHCR is required.
            registry: Registry::Ghcr,
            org: "stakwork".to_string(),
            repo: "stakgraph-gateway".to_string(),
            root_volume: "/app/data".to_string(),
        }
    }
}

pub fn bifrost(
    img: &BifrostImage,
    boltwall: &Option<BoltwallImage>,
    redis: &Option<RedisImage>,
) -> Config<String> {
    let name = img.name.clone();
    let repo = img.repo();
    let image = img.image();
    let root_vol = repo.root_volume.clone();
    let ports = vec![img.port.clone()];

    // NB: do NOT set APP_PORT / APP_HOST here. The stakgraph-gateway
    // image's wrapper binary (PID 1) is the public listener — it
    // passes `-host 127.0.0.1 -port 8080` as flags to bifrost-http,
    // which take precedence over APP_HOST/APP_PORT. The Dockerfile
    // bakes in `APP_PORT=8080` / `APP_HOST=127.0.0.1` already; setting
    // APP_PORT=8181 here would be ignored but misleading.
    let mut env: Vec<String> = vec![];

    // Admin credentials for Bifrost's auth_config (config.json references
    // env.BIFROST_ADMIN_USER / env.BIFROST_ADMIN_PASS). Bifrost bcrypts
    // the password on first boot and persists the hash; rotating the env
    // value flushes existing sessions and accepts the new password.
    env.push(format!("BIFROST_ADMIN_USER={}", img.admin_user));
    env.push(format!("BIFROST_ADMIN_PASS={}", img.admin_password));

    // Provisioning token for the in-process plugin server's
    // /_plugin/admin-credentials endpoint. Same value Boltwall already
    // shares with repo2graph (`stakwork_secret`). Hive's backend presents
    // it as a Bearer token to bootstrap itself with the admin password.
    if let Some(boltwall) = boltwall {
        if let Some(api_token) = &boltwall.stakwork_secret {
            env.push(format!("BIFROST_PROVISIONING_TOKEN={}", api_token));
        }
    }

    // Plugin's Redis connection for macaroon enforcement: revocation
    // checks, per-run cost accumulators, kill switches, tool-loop
    // history. The plugin shares the swarm's redis.sphinx instance
    // with Jarvis; keys are namespaced with the fixed `bifrost:`
    // prefix (see gateway/plans/phases/phase-6-plugin-enforcement.md
    // "Namespace") so the two consumers' keyspaces don't collide.
    //
    // Logical DB 0 is shared with Jarvis intentionally — the prefix
    // is the partitioning mechanism, not the DB number. This keeps
    // the plugin's keyspace inspectable with `SCAN MATCH bifrost:*`
    // from any redis-cli pointed at the standard instance.
    //
    // Absent redis link ⇒ no env var ⇒ plugin runs in observability
    // mode (verifies signatures, skips Redis-backed enforcement).
    // This matches the phase-5 "trust registry wired but enforcement
    // off" stance and keeps stacks without redis bootable.
    if let Some(redis) = redis {
        env.push(format!(
            "BIFROST_PLUGIN_REDIS_URL=redis://{}:{}/0",
            domain(&redis.name),
            redis.http_port,
        ));
    }

    // Provider API keys referenced by Bifrost's config.json via env.<NAME>.
    if let Ok(openai_api_key) = getenv("OPENAI_API_KEY") {
        env.push(format!("OPENAI_API_KEY={}", openai_api_key));
    }
    if let Ok(anthropic_api_key) = getenv("ANTHROPIC_API_KEY") {
        env.push(format!("ANTHROPIC_API_KEY={}", anthropic_api_key));
    }
    if let Ok(openrouter_api_key) = getenv("OPENROUTER_API_KEY") {
        env.push(format!("OPENROUTER_API_KEY={}", openrouter_api_key));
    }
    if let Ok(google_api_key) = getenv("GOOGLE_API_KEY") {
        env.push(format!("GOOGLE_API_KEY={}", google_api_key));
    }

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        // `host_config` already mounts `<name>:/app/data` (the repo's
        // root_volume) as the first bind, which persists Bifrost's
        // config + state. Don't pass an extra `bifrost-data:/app/data`
        // bind — Docker rejects duplicate mount points with HTTP 400.
        host_config: host_config(&name, ports, &root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };

    if let Some(host) = img.host.clone() {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false));
    }

    c
}

fn strarr(i: Vec<&str>) -> Vec<String> {
    i.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_bifrost_image() -> BifrostImage {
        BifrostImage::new("bifrost", "latest")
    }

    fn test_boltwall_with_secret(secret: &str) -> BoltwallImage {
        let mut bw = BoltwallImage::new("boltwall", "latest", "8444");
        bw.stakwork_secret = Some(secret.to_string());
        bw
    }

    fn test_redis() -> RedisImage {
        RedisImage::new("redis", "latest")
    }

    #[test]
    fn test_bifrost_image_uses_stakgraph_gateway_repo() {
        let img = test_bifrost_image();
        let repo = img.repo();
        assert!(matches!(repo.registry, Registry::Ghcr));
        assert_eq!(repo.org, "stakwork");
        assert_eq!(repo.repo, "stakgraph-gateway");
        assert_eq!(repo.root_volume, "/app/data");
    }

    #[test]
    fn test_bifrost_admin_credentials_generated() {
        let img = test_bifrost_image();
        assert_eq!(img.admin_user, "admin");
        // 16 chars from [A-Za-z0-9]
        assert_eq!(img.admin_password.len(), 16);
        assert!(img
            .admin_password
            .chars()
            .all(|c| c.is_ascii_alphanumeric()));

        // Each construction generates a fresh password.
        let img2 = test_bifrost_image();
        assert_ne!(img.admin_password, img2.admin_password);
    }

    #[test]
    fn test_bifrost_env_with_keys() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::set_var("OPENAI_API_KEY", "test-openai-key");
        std::env::set_var("ANTHROPIC_API_KEY", "test-anthropic-key");
        std::env::set_var("OPENROUTER_API_KEY", "test-openrouter-key");
        std::env::set_var("GOOGLE_API_KEY", "test-google-key");

        let img = test_bifrost_image();
        let config = bifrost(&img, &None, &None);
        let env = config.env.unwrap();

        assert!(env.contains(&format!("BIFROST_ADMIN_USER={}", img.admin_user)));
        assert!(env.contains(&format!("BIFROST_ADMIN_PASS={}", img.admin_password)));
        assert!(env.contains(&"OPENAI_API_KEY=test-openai-key".to_string()));
        assert!(env.contains(&"ANTHROPIC_API_KEY=test-anthropic-key".to_string()));
        assert!(env.contains(&"OPENROUTER_API_KEY=test-openrouter-key".to_string()));
        assert!(env.contains(&"GOOGLE_API_KEY=test-google-key".to_string()));

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");
    }

    #[test]
    fn test_bifrost_env_without_keys() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");

        let img = test_bifrost_image();
        let config = bifrost(&img, &None, &None);
        let env = config.env.unwrap();

        assert_eq!(
            env,
            vec![
                format!("BIFROST_ADMIN_USER={}", img.admin_user),
                format!("BIFROST_ADMIN_PASS={}", img.admin_password),
            ]
        );
    }

    #[test]
    fn test_bifrost_provisioning_token_from_boltwall() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");

        let img = test_bifrost_image();
        let boltwall = Some(test_boltwall_with_secret("stakwork-shared-secret"));
        let config = bifrost(&img, &boltwall, &None);
        let env = config.env.unwrap();

        assert!(env.contains(&"BIFROST_PROVISIONING_TOKEN=stakwork-shared-secret".to_string()));
    }

    #[test]
    fn test_bifrost_no_provisioning_token_without_boltwall() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");

        let img = test_bifrost_image();
        let config = bifrost(&img, &None, &None);
        let env = config.env.unwrap();

        assert!(!env
            .iter()
            .any(|e| e.starts_with("BIFROST_PROVISIONING_TOKEN=")));
    }

    #[test]
    fn test_bifrost_redis_url_emitted_when_linked() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");

        let img = test_bifrost_image();
        let redis = Some(test_redis());
        let config = bifrost(&img, &None, &redis);
        let env = config.env.unwrap();

        // Shape comes from images/redis.rs: name="redis", http_port="6379".
        // domain() resolves to the container hostname inside the
        // swarm's docker network. Logical DB 0 is shared with Jarvis;
        // the bifrost: key prefix (phase-6 "Namespace") is what
        // partitions the keyspace.
        assert!(env.contains(&"BIFROST_PLUGIN_REDIS_URL=redis://redis.sphinx:6379/0".to_string()));
    }

    #[test]
    fn test_bifrost_no_redis_url_without_link() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");

        let img = test_bifrost_image();
        let config = bifrost(&img, &None, &None);
        let env = config.env.unwrap();

        // Absent redis link ⇒ plugin runs in observability mode (no
        // Redis-backed revocation / budget enforcement).
        assert!(!env
            .iter()
            .any(|e| e.starts_with("BIFROST_PLUGIN_REDIS_URL=")));
    }
}
