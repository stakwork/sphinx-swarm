use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::broker::BrokerImage;
use crate::images::tribes::TribesImage;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BotImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub seed: String,
    pub admin_token: String,
    pub host: Option<String>,
    pub router_url: Option<String>,
    pub initial_delay: Option<String>,
    pub links: Links,
}

impl BotImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            seed: crate::secrets::hex_secret_32(),
            admin_token: crate::secrets::hex_secret_32(),
            host: None,
            router_url: None,
            initial_delay: None,
            links: vec![],
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
    pub fn set_admin_token(&mut self, at: &str) {
        self.admin_token = at.to_string();
    }
    pub fn set_router_url(&mut self, ru: &str) {
        self.router_url = Some(ru.to_string());
    }
    pub fn set_initial_delay(&mut self, id: &str) {
        self.initial_delay = Some(id.to_string())
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for BotImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let broker = li.find_broker().context("Bot: No Broker")?;
        let tribes = li.find_tribes();
        Ok(bot(self, &broker, &tribes)?)
    }
}

impl DockerHubImage for BotImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "sphinx-bot".to_string(),
            root_volume: "/home/.bot".to_string(),
        }
    }
}

fn bot(
    img: &BotImage,
    broker: &BrokerImage,
    tribes_opt: &Option<TribesImage>,
) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("MY_ALIAS={}", "bot"),
        format!("PORT={}", img.port),
        format!("SEED={}", img.seed),
        format!("ADMIN_TOKEN={}", img.admin_token),
        format!("STORE_FILE={}", "/home/.bot/db"),
        format!(
            "BROKER=http://{}:{}",
            domain(&broker.name),
            broker.mqtt_port
        ),
    ];
    if let Some(tribes) = tribes_opt {
        env.push(format!(
            "TRIBES_URL=http://{}:{}",
            domain(&tribes.name),
            tribes.port
        ));
    }
    if let Some(router_url) = &img.router_url {
        env.push(format!("ROUTER_URL={}", router_url));
    }
    if let Some(initial_delay) = &img.initial_delay {
        env.push(format!("INITIAL_DELAY={}", initial_delay));
    }

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };
    if let Some(host) = &img.host {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false))
    }
    Ok(c)
}
