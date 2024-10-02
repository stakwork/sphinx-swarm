use super::*;
use crate::config::Node;
use crate::images::whisper::WhisperImage;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

const MODEL: &str = "Systran/faster-distil-whisper-large-v3";

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct WhiskerImage {
    pub name: String,
    pub version: String,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub model: Option<String>,
    pub links: Links,
}

impl WhiskerImage {
    pub fn new(
        name: &str,
        version: &str,
        livekit_url: &str,
        livekit_api_key: &str,
        livekit_api_secret: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            livekit_url: livekit_url.to_string(),
            livekit_api_key: livekit_api_key.to_string(),
            livekit_api_secret: livekit_api_secret.to_string(),
            model: None,
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for WhiskerImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let whisper = li.find_whisper().context("Whisper not found")?;
        Ok(whisker(self, &whisper)?)
    }
}

impl DockerHubImage for WhiskerImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "whisker".to_string(),
            root_volume: "/root".to_string(),
        }
    }
}

fn whisker(img: &WhiskerImage, whisper: &WhisperImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = &repo.root_volume;

    let env = vec![
        format!("MODEL={MODEL}"),
        format!("LIVEKIT_URL={}", img.livekit_url),
        format!("LIVEKIT_API_KEY={}", img.livekit_api_key),
        format!("LIVEKIT_API_SECRET={}", img.livekit_api_secret),
        format!(
            "BASE_URL_WS=http://{}:{}",
            domain(&whisper.name),
            whisper.port
        ),
    ];

    let c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(Vec::new()),
        host_config: host_config(&img.name, Vec::new(), root_vol, None, None),
        env: Some(env),
        ..Default::default()
    };
    Ok(c)
}
