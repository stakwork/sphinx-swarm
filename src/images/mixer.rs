use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::broker::BrokerImage;
use crate::images::cln::ClnImage;
use crate::utils::{domain, exposed_ports, getenv, host_config, volume_string};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct MixerImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub port: String,
    pub no_lightning: Option<bool>,
    pub no_gateway: Option<bool>,
    pub no_mqtt: Option<bool>,
    pub host: Option<String>,
    pub links: Links,
    pub log_level: Option<String>,
    pub initial_peers: Option<String>, // alt brokers
    pub default_tribe_pubkey: Option<String>,
    pub default_router: Option<String>,
    pub hub_url: Option<String>,
    pub invite_price: Option<String>,
    pub timeout_ms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purge_msgs_days: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_compact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_integrity_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_workflow_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_workflow_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_workflow_timeout_secs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_workflow_app_ids: Option<String>,
}

impl std::fmt::Debug for MixerImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = self.push_workflow_key.as_ref().map(|_| "<redacted>");
        f.debug_struct("MixerImage")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("network", &self.network)
            .field("port", &self.port)
            .field("no_lightning", &self.no_lightning)
            .field("no_gateway", &self.no_gateway)
            .field("no_mqtt", &self.no_mqtt)
            .field("host", &self.host)
            .field("links", &self.links)
            .field("log_level", &self.log_level)
            .field("initial_peers", &self.initial_peers)
            .field("default_tribe_pubkey", &self.default_tribe_pubkey)
            .field("default_router", &self.default_router)
            .field("hub_url", &self.hub_url)
            .field("invite_price", &self.invite_price)
            .field("timeout_ms", &self.timeout_ms)
            .field("purge_msgs_days", &self.purge_msgs_days)
            .field("db_compact", &self.db_compact)
            .field("db_integrity_check", &self.db_integrity_check)
            .field("push_workflow_url", &self.push_workflow_url)
            .field("push_workflow_key", &key)
            .field("push_workflow_timeout_secs", &self.push_workflow_timeout_secs)
            .field("push_workflow_app_ids", &self.push_workflow_app_ids)
            .finish()
    }
}

impl MixerImage {
    pub fn new(name: &str, version: &str, network: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            port: port.to_string(),
            no_lightning: None,
            no_gateway: None,
            no_mqtt: None,
            links: vec![],
            host: None,
            log_level: None,
            initial_peers: None,
            default_tribe_pubkey: None,
            default_router: None,
            hub_url: None,
            invite_price: None,
            timeout_ms: None,
            purge_msgs_days: None,
            db_compact: None,
            db_integrity_check: None,
            push_workflow_url: None,
            push_workflow_key: None,
            push_workflow_timeout_secs: None,
            push_workflow_app_ids: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn set_no_lightning(&mut self) {
        self.no_lightning = Some(true)
    }
    pub fn set_no_gateway(&mut self) {
        self.no_gateway = Some(true)
    }
    pub fn set_no_mqtt(&mut self) {
        self.no_mqtt = Some(true)
    }
    pub fn set_log_level(&mut self, log_level: &str) {
        self.log_level = Some(log_level.to_string())
    }
    pub fn set_initial_peers(&mut self, peers: &str) {
        self.initial_peers = Some(peers.to_string())
    }
    pub fn set_default_tribe_pubkey(&mut self, pk: &str) {
        self.default_tribe_pubkey = Some(pk.to_string())
    }
    pub fn set_default_router(&mut self, dr: &str) {
        self.default_router = Some(dr.to_string())
    }
    pub fn set_hub_url(&mut self, hu: &str) {
        self.hub_url = Some(hu.to_string())
    }
    pub fn set_push_workflow_url(&mut self, v: &str) {
        self.push_workflow_url = Some(v.to_string())
    }
    pub fn set_push_workflow_key(&mut self, v: &str) {
        self.push_workflow_key = Some(v.to_string())
    }
    pub fn set_push_workflow_timeout_secs(&mut self, v: &str) {
        self.push_workflow_timeout_secs = Some(v.to_string())
    }
    pub fn set_push_workflow_app_ids(&mut self, v: &str) {
        self.push_workflow_app_ids = Some(v.to_string())
    }
    pub fn set_purge_msgs_days(&mut self, days: &str) {
        self.purge_msgs_days = Some(days.to_string())
    }
    pub fn set_db_compact(&mut self) {
        self.db_compact = Some(true)
    }
    pub fn set_db_integrity_check(&mut self) {
        self.db_integrity_check = Some(true)
    }
}

#[async_trait]
impl DockerConfig for MixerImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let broker = li.find_broker().context("Mixer: No Broker")?;
        let cln = li.find_cln();
        Ok(mixer(self, &broker, &cln)?)
    }
}

impl DockerHubImage for MixerImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "sphinx-mixer".to_string(),
            root_volume: "/home".to_string(),
        }
    }
}

fn mixer(img: &MixerImage, broker: &BrokerImage, cln: &Option<ClnImage>) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("SEED={}", broker.seed),
        format!("DB_PATH=/home/data"),
        format!("ROCKET_ADDRESS=0.0.0.0"),
        format!("ROCKET_PORT={}", img.port),
    ];

    let mut extra_vols = Vec::new();

    if bool_arg(&img.no_gateway) {
        env.push("NO_GATEWAY=true".to_string());
    } else if let Some(c) = cln {
        env.push(format!("GATEWAY_IP={}", domain(&c.name)));
        // gateway grpc port is the normal grpc port + 200
        let grpc_port: u16 = c.grpc_port.parse::<u16>()?;
        env.push(format!("GATEWAY_PORT={}", grpc_port + 200));
    }

    if bool_arg(&img.no_lightning) {
        env.push("NO_LIGHTNING=true".to_string());
    } else if let Some(c) = cln {
        let cln_vol = volume_string(&c.name, "/cln");
        extra_vols.push(cln_vol);
        let creds = c.credentials_paths("cln");
        env.push(format!("CA_PEM={}", creds.ca_cert));
        env.push(format!("CLIENT_PEM={}", creds.client_cert));
        env.push(format!("KEY_PEM={}", creds.client_key));
        env.push(format!("CLN_IP={}", domain(&c.name)));
        env.push(format!("CLN_PORT={}", &c.grpc_port));
    }

    if bool_arg(&img.no_mqtt) {
        env.push("NO_MQTT=true".to_string());
    } else {
        let bu = format!("{}:{}", domain(&broker.name), broker.mqtt_port);
        env.push(format!("BROKER_URL={}", bu));
    }

    if let Some(ll) = &img.log_level {
        env.push(format!("RUST_LOG={}", ll));
    }

    if let Some(ips) = &img.initial_peers {
        env.push(format!("INITIAL_PEERS={}", ips));
    }

    if let Some(ip) = &img.invite_price {
        env.push(format!("INVITE_PRICE={}", ip));
    }

    if let Some(tm) = &img.timeout_ms {
        env.push(format!("TIMEOUT_MS={}", tm));
    }

    if let Some(pd) = &img.purge_msgs_days {
        env.push(format!("PURGE_MSGS_DAYS={}", pd));
    }

    if bool_arg(&img.db_compact) {
        env.push("DB_COMPACT=true".to_string());
    }

    if bool_arg(&img.db_integrity_check) {
        env.push("DB_INTEGRITY_CHECK=true".to_string());
    }

    if let Ok(toats) = std::env::var("TESTING_ONLY_ADD_TO_SENDER") {
        if toats == "1" {
            env.push(format!("TESTING_ONLY_ADD_TO_SENDER=1"));
        }
    }

    if let Some(dts) = &img.default_tribe_pubkey {
        env.push(format!("DEFAULT_TRIBE_PUBKEY={}", dts));
    }
    if let Some(dr) = &img.default_router {
        env.push(format!("DEFAULT_ROUTER={}", dr));
    }
    if let Some(hu) = &img.hub_url {
        env.push(format!("HUB_URL={}", hu));
    }
    if let Some(v) = &img.push_workflow_url {
        env.push(format!("PUSH_WORKFLOW_URL={}", v));
    }
    if let Some(v) = &img.push_workflow_key {
        env.push(format!("PUSH_WORKFLOW_KEY={}", v));
    }
    if let Some(v) = &img.push_workflow_timeout_secs {
        env.push(format!("PUSH_WORKFLOW_TIMEOUT_SECS={}", v));
    }
    if let Some(v) = &img.push_workflow_app_ids {
        env.push(format!("PUSH_WORKFLOW_APP_IDS={}", v));
    }

    let mut image = format!("{}:{}", image, img.version);
    if let Ok(ci) = getenv("LOCAL_MIXER_IMG") {
        if ci == "true" || ci == "1" {
            image = "sphinx-mixer:latest".to_string();
        }
    }
    let mut c = Config {
        image: Some(image),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, Some(extra_vols), None),
        env: Some(env),
        ..Default::default()
    };
    if let Some(host) = &img.host {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false))
    }
    Ok(c)
}

fn bool_arg(arg: &Option<bool>) -> bool {
    if let Some(nl) = arg {
        return nl.clone();
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_push_workflow_key() {
        let mut img = MixerImage::new("mixer", "v1", "regtest", "8080");
        img.push_workflow_key = Some("supersecret".to_string());
        let out = format!("{:?}", img);
        assert!(!out.contains("supersecret"));
        assert!(out.contains("<redacted>"));
        img.push_workflow_key = None;
        let out = format!("{:?}", img);
        assert!(out.contains("push_workflow_key: None"));
    }
}
