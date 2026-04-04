use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::boltwall::BoltwallImage;
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
    pub external_broker: Option<String>,
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
            external_broker: None,
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
    pub fn set_external_broker(&mut self, url: &str) {
        self.external_broker = Some(url.to_string());
    }
    /// Resolve the actual admin token used by the running container.
    /// If a linked BoltWall has a swarm_api_token, that overrides the bot's own token.
    pub fn actual_admin_token(&self, boltwall: &Option<BoltwallImage>) -> String {
        boltwall
            .as_ref()
            .and_then(|b| b.swarm_api_token.clone())
            .unwrap_or_else(|| self.admin_token.clone())
    }
}

#[async_trait]
impl DockerConfig for BotImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let broker = if self.external_broker.is_some() {
            None
        } else {
            Some(li.find_broker().context("Bot: No Broker")?)
        };
        let tribes = li.find_tribes();
        let boltwall = li.find_boltwall();
        Ok(bot(self, broker.as_ref(), &tribes, &boltwall)?)
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
    broker: Option<&BrokerImage>,
    tribes_opt: &Option<TribesImage>,
    boltwall: &Option<BoltwallImage>,
) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let admin_token = img.actual_admin_token(boltwall);

    let broker_url = if let Some(b) = broker {
        format!("http://{}:{}", domain(&b.name), b.mqtt_port)
    } else {
        img.external_broker.clone().unwrap_or_default()
    };

    let mut env = vec![
        format!("MY_ALIAS={}", "bot"),
        format!("PORT={}", img.port),
        format!("SEED={}", img.seed),
        format!("ADMIN_TOKEN={}", admin_token),
        format!("STORE_FILE={}", "/home/.bot/db"),
        format!("BROKER={}", broker_url),
        "NETWORK=bitcoin".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::boltwall::BoltwallImage;
    use crate::images::broker::BrokerImage;

    fn make_bot() -> BotImage {
        let mut b = BotImage::new("test-bot", "latest", "3000");
        b.admin_token = "bot-own-token".to_string();
        b
    }

    fn make_broker() -> BrokerImage {
        BrokerImage::new("broker", "latest", "regtest", "1883", None)
    }

    #[test]
    fn test_admin_token_uses_bot_own_when_no_boltwall() {
        let img = make_bot();
        let broker = make_broker();
        let config = bot(&img, Some(&broker), &None, &None).unwrap();
        let env = config.env.unwrap();
        assert!(
            env.contains(&"ADMIN_TOKEN=bot-own-token".to_string()),
            "Expected ADMIN_TOKEN to equal bot's own admin_token"
        );
    }

    #[test]
    fn test_admin_token_uses_boltwall_swarm_api_token_when_present() {
        let img = make_bot();
        let broker = make_broker();
        let mut boltwall = BoltwallImage::new("boltwall", "latest", "8444");
        boltwall.swarm_api_token = Some("boltwall-secret".to_string());
        let config = bot(&img, Some(&broker), &None, &Some(boltwall)).unwrap();
        let env = config.env.unwrap();
        assert!(
            env.contains(&"ADMIN_TOKEN=boltwall-secret".to_string()),
            "Expected ADMIN_TOKEN to equal boltwall swarm_api_token"
        );
    }

    #[test]
    fn test_admin_token_falls_back_to_bot_when_boltwall_swarm_api_token_is_none() {
        let img = make_bot();
        let broker = make_broker();
        let mut boltwall = BoltwallImage::new("boltwall", "latest", "8444");
        boltwall.swarm_api_token = None;
        let config = bot(&img, Some(&broker), &None, &Some(boltwall)).unwrap();
        let env = config.env.unwrap();
        assert!(
            env.contains(&"ADMIN_TOKEN=bot-own-token".to_string()),
            "Expected ADMIN_TOKEN to fall back to bot's own admin_token when swarm_api_token is None"
        );
    }
}
