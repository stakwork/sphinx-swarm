use super::*;
use crate::rsa;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config, user};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub log: bool,
    pub priv_key: String,
    pub rsa_key: String,
    pub links: Links,
}

impl CacheImage {
    pub fn new(name: &str, version: &str, port: &str, log: bool) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            links: vec![],
            log,
            priv_key: secrets::hex_secret_32(),
            rsa_key: rsa::rand_key(),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}
impl DockerHubImage for CacheImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-cache".to_string(),
        }
    }
}

pub fn cache(project: &str, node: &CacheImage, meme_host: &str, mqtt_host: &str) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/cache/data";
    let ports = vec![node.port.clone()];
    Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        user: user(),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(project, &name, ports, root_vol, None, None),
        env: Some(vec![
            format!("PRIVATE_KEY={}", node.priv_key),
            format!("MQTT_HOST={}", mqtt_host),
            "MQTT_PORT=1883".to_string(),
            "MQTT_CLIENT_ID=local-123".to_string(),
            format!("LOG_INCOMING={}", node.log),
            format!("RSA_KEY={}", node.rsa_key),
            format!("MEME_HOST={}", meme_host),
        ]),
        ..Default::default()
    }
}
