use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
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
}

impl BifrostImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: "8181".to_string(),
            links: vec![],
            host: None,
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
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        Ok(bifrost(self))
    }
}

impl DockerHubImage for BifrostImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "maximhq".to_string(),
            repo: "bifrost".to_string(),
            root_volume: "/app/data".to_string(),
        }
    }
}

pub fn bifrost(img: &BifrostImage) -> Config<String> {
    let name = img.name.clone();
    let repo = img.repo();
    let image = img.image();
    let root_vol = repo.root_volume.clone();
    let ports = vec![img.port.clone()];

    let mut env = vec![format!("PORT={}", img.port)];

    if let Ok(openai_api_key) = getenv("OPENAI_API_KEY") {
        env.push(format!("OPENAI_API_KEY={}", openai_api_key));
    }
    if let Ok(anthropic_api_key) = getenv("ANTHROPIC_API_KEY") {
        env.push(format!("ANTHROPIC_API_KEY={}", anthropic_api_key));
    }
    if let Ok(openrouter_api_key) = getenv("OPENROUTER_API_KEY") {
        env.push(format!("OPENROUTER_API_KEY={}", openrouter_api_key));
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

    #[test]
    fn test_bifrost_env_with_keys() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::set_var("OPENAI_API_KEY", "test-openai-key");
        std::env::set_var("ANTHROPIC_API_KEY", "test-anthropic-key");
        std::env::set_var("OPENROUTER_API_KEY", "test-openrouter-key");

        let img = test_bifrost_image();
        let config = bifrost(&img);
        let env = config.env.unwrap();

        assert!(env.contains(&"PORT=8181".to_string()));
        assert!(env.contains(&"OPENAI_API_KEY=test-openai-key".to_string()));
        assert!(env.contains(&"ANTHROPIC_API_KEY=test-anthropic-key".to_string()));
        assert!(env.contains(&"OPENROUTER_API_KEY=test-openrouter-key".to_string()));

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");
    }

    #[test]
    fn test_bifrost_env_without_keys() {
        let _lock = ENV_LOCK.lock().unwrap();

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OPENROUTER_API_KEY");

        let img = test_bifrost_image();
        let config = bifrost(&img);
        let env = config.env.unwrap();

        assert_eq!(env, vec!["PORT=8181".to_string()]);
    }
}
