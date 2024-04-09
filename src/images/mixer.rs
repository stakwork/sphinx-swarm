use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::broker::BrokerImage;
use crate::images::cln::ClnImage;
use crate::utils::{domain, exposed_ports, host_config, volume_string};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
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
            org: "sphinxlightning".to_string(),
            repo: "sphinx-mixer".to_string(),
        }
    }
}

fn mixer(img: &MixerImage, broker: &BrokerImage, cln: &Option<ClnImage>) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = "/home";

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

    if let Ok(toats) = std::env::var("TESTING_ONLY_ADD_TO_SENDER") {
        if toats == "1" {
            env.push(format!("TESTING_ONLY_ADD_TO_SENDER=1"));
        }
    }

    if let Some(dts) = &img.default_tribe_pubkey {
        env.push(format!("DEFAULT_TRIBE_PUBKEY={}", dts));
    }

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
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
