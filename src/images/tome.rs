use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::bot::BotImage;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TomeImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub jwt_secret: String,
    pub add_msat_per_gb: u64,
    pub upload_msat_per_gb: u64,
    pub storage_msat_per_gb_per_month: u64,
    pub host: Option<String>,
    pub links: Links,
}

impl TomeImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            jwt_secret: crate::secrets::hex_secret_32(),
            add_msat_per_gb: 0,
            upload_msat_per_gb: 0,
            storage_msat_per_gb_per_month: 0,
            host: None,
            links: vec![],
        }
    }
    pub fn set_add_msat_per_gb(&mut self, msat: u64) {
        self.add_msat_per_gb = msat;
    }
    pub fn set_upload_msat_per_gb(&mut self, msat: u64) {
        self.upload_msat_per_gb = msat;
    }
    pub fn set_storage_msat_per_gb_per_month(&mut self, msat: u64) {
        self.storage_msat_per_gb_per_month = msat;
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for TomeImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let bot = li.find_bot();
        let rqbit = RqbitImage {
            name: "rqbit".to_string(),
            port: "3030".to_string(),
        };
        Ok(tome(self, &bot, &Some(rqbit))?)
    }
}

impl DockerHubImage for TomeImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "tome".to_string(),
            root_volume: "/home/.tome".to_string(),
        }
    }
}

pub struct RqbitImage {
    pub name: String,
    pub port: String,
}

fn tome(
    img: &TomeImage,
    bot_opt: &Option<BotImage>,
    rqbit_opt: &Option<RqbitImage>,
) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let mut env = vec![
        format!("PORT={}", img.port),
        format!("JWT_SECRET={}", img.jwt_secret),
        format!("ADD_MSAT_PER_GB={}", img.add_msat_per_gb),
        format!("UPLOAD_MSAT_PER_GB={}", img.upload_msat_per_gb),
        format!(
            "STORAGE_MSAT_PER_GB_PER_MONTH={}",
            img.storage_msat_per_gb_per_month
        ),
    ];
    if let Some(bot) = bot_opt {
        env.push(format!("BOT_URL=http://{}:{}", domain(&bot.name), bot.port));
        env.push(format!("BOT_TOKEN={}", bot.admin_token));
    }
    if let Some(rqbit) = rqbit_opt {
        env.push(format!(
            "RQBIT_URL=http://{}:{}",
            domain(&rqbit.name),
            rqbit.port
        ));
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
