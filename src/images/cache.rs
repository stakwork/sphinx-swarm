use super::*;
use crate::config::{ExternalNodeType, Node};
use crate::rsa;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};
use url::{Host, Url};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct CacheImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub log: bool,
    pub priv_key: String,
    pub rsa_key: String,
    pub client_id: String,
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
            client_id: format!("sphinx-{}", secrets::random_word(8)),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for CacheImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let memes = nodes
            .iter()
            .find(|n| n.is_ext_of_type(ExternalNodeType::Meme))
            .context("No Memes")?
            .as_external()?;

        let memes_url = Url::parse(format!("https://{}", memes.url).as_str())?;
        let memes_host = memes_url.host().unwrap_or(Host::Domain("")).to_string();

        let tribes = nodes
            .iter()
            .find(|n| n.is_ext_of_type(ExternalNodeType::Tribes))
            .context("No Tribes")?
            .as_external()?;

        let tribes_url = Url::parse(format!("https://{}", tribes.url).as_str())?;
        let tribe_host = tribes_url.host().unwrap_or(Host::Domain("")).to_string();

        let secure_mqtt_port = "8883";
        println!("mqtt host {}", tribe_host);
        Ok(cache(&self, &memes_host, &tribe_host, secure_mqtt_port))
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

fn cache(node: &CacheImage, meme_host: &str, mqtt_host: &str, mqtt_port: &str) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let root_vol = "/cache/data";
    let ports = vec![node.port.clone()];
    Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, None, None),
        env: Some(vec![
            format!("PRIVATE_KEY={}", node.priv_key),
            format!("MQTT_HOST={}", mqtt_host),
            format!("MQTT_PORT={}", mqtt_port),
            format!("MQTT_CLIENT_ID={}", node.client_id),
            format!("LOG_INCOMING={}", node.log),
            format!("RSA_KEY={}", node.rsa_key),
            format!("MEME_HOST={}", meme_host),
        ]),
        ..Default::default()
    }
}
