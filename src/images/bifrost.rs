use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::boltwall::BoltwallImage;
use crate::secrets;
use crate::utils::{domain, exposed_ports, getenv, host_config, volume_string};
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
        Ok(bifrost(self, &boltwall))
    }
}

impl DockerHubImage for BifrostImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "stakgraph-gateway".to_string(),
            root_volume: "/app/data".to_string(),
        }
    }
}

pub fn bifrost(img: &BifrostImage, boltwall: &Option<BoltwallImage>) -> Config<String> {
    let name = img.name.clone();
    let repo = img.repo();
    let image = img.image();
    let root_vol = repo.root_volume.clone();
    let ports = vec![img.port.clone()];

    // Bifrost looks for APP_PORT / APP_HOST, NOT PORT. Without these it
    // listens on localhost:8080 inside the container, which makes the host
    // port binding useless.
    let mut env = vec![
        format!("APP_PORT={}", img.port),
        "APP_HOST=0.0.0.0".to_string(),
    ];

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

    let data_vol = volume_string(&format!("{}-data", name), &root_vol);
    let extra_vols = vec![data_vol];

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, &root_vol, Some(extra_vols), None),
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

    #[test]
    fn test_bifrost_image_uses_stakgraph_gateway_repo() {
        let img = test_bifrost_image();
        let repo = img.repo();
        assert_eq!(repo.org, "sphinxlightning");
        assert_eq!(repo.repo, "stakgraph-gateway");
        assert_eq!(repo.root_volume, "/app/data");
    }

    #[test]
    fn test_bifrost_admin_credentials_generated() {
        let img = test_bifrost_image();
        assert_eq!(img.admin_user, "admin");
        // 16 chars from [A-Za-z0-9]
        assert_eq!(img.admin_password.len(), 16);
        assert!(img.admin_password.chars().all(|c| c.is_ascii_alphanumeric()));

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
        let config = bifrost(&img, &None);
        let env = config.env.unwrap();

        assert!(env.contains(&"APP_PORT=8181".to_string()));
        assert!(env.contains(&"APP_HOST=0.0.0.0".to_string()));
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
        let config = bifrost(&img, &None);
        let env = config.env.unwrap();

        assert_eq!(
            env,
            vec![
                "APP_PORT=8181".to_string(),
                "APP_HOST=0.0.0.0".to_string(),
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
        let config = bifrost(&img, &boltwall);
        let env = config.env.unwrap();

        assert!(env.contains(
            &"BIFROST_PROVISIONING_TOKEN=stakwork-shared-secret".to_string()
        ));
    }

    #[test]
    fn test_bifrost_no_provisioning_token_without_boltwall() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");

        let img = test_bifrost_image();
        let config = bifrost(&img, &None);
        let env = config.env.unwrap();

        assert!(!env.iter().any(|e| e.starts_with("BIFROST_PROVISIONING_TOKEN=")));
    }
}
