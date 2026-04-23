use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::boltwall::BoltwallImage;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct HiveRelayImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
}

impl HiveRelayImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: "3333".to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("hive-relay.{}", h));
        }
    }
}

#[async_trait]
impl DockerConfig for HiveRelayImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let boltwall = li.find_boltwall();
        Ok(hive_relay(self, &boltwall))
    }
}

impl DockerHubImage for HiveRelayImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "hive-relay".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

pub fn hive_relay(img: &HiveRelayImage, boltwall: &Option<BoltwallImage>) -> Config<String> {
    let name = img.name.clone();
    let repo = img.repo();
    let image = img.image();
    let root_vol = repo.root_volume;
    let ports = vec![img.port.clone()];

    let mut env = vec![format!("PORT={}", img.port)];
    if let Some(bw) = boltwall {
        if let Some(api_token) = &bw.stakwork_secret {
            env.push(format!("SWARM_API_KEY={}", api_token));
        }
    }

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, &root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };

    if let Some(host) = img.host.clone() {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, true));
    }

    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::boltwall::BoltwallImage;

    fn test_hive_relay_image() -> HiveRelayImage {
        HiveRelayImage::new("hive-relay", "latest")
    }

    #[test]
    fn test_hive_relay_env_with_boltwall_secret() {
        let img = test_hive_relay_image();
        let mut bw = BoltwallImage::new("boltwall", "latest", "8444");
        bw.stakwork_secret = Some("test-secret-123".to_string());
        let config = hive_relay(&img, &Some(bw));
        let env = config.env.unwrap();
        assert!(env.contains(&"PORT=3333".to_string()));
        assert!(env.contains(&"SWARM_API_KEY=test-secret-123".to_string()));
    }

    #[test]
    fn test_hive_relay_env_without_boltwall() {
        let img = test_hive_relay_image();
        let config = hive_relay(&img, &None);
        let env = config.env.unwrap();
        assert_eq!(env, vec!["PORT=3333".to_string()]);
    }
}
