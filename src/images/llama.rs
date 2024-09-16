use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct LlamaImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub model: String,
    pub host: Option<String>,
    pub links: Links,
}

// https://huggingface.co/TheBloke/Llama-2-7B-GGUF
const DEFAULT_MODEL: &str = "models/llama-2-7b.Q4_K_M.gguf";

impl LlamaImage {
    pub fn new(name: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "server".to_string(),
            port: port.to_string(),
            model: DEFAULT_MODEL.to_string(),
            host: None,
            links: Vec::new(),
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
}

#[async_trait]
impl DockerConfig for LlamaImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        // let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        Ok(llama(self)?)
    }
}

impl DockerHubImage for LlamaImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "ggerganov".to_string(),
            repo: "llama.cpp".to_string(),
            root_volume: "/home/llama".to_string(),
        }
    }
}

fn get_current_working_dir() -> std::io::Result<PathBuf> {
    env::current_dir()
}

fn llama(img: &LlamaImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image_end = format!("{}/{}", repo.org, repo.repo);
    let image = format!("ghcr.io/{}", image_end);

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let model_path = format!("/{}", img.model);
    let env = vec![
        format!("LLAMA_ARG_PORT={}", img.port),
        format!("LLAMA_ARG_MODEL={}", model_path),
    ];

    let cwd = get_current_working_dir()?;
    let model_vol = format!("{}/{}:/{}", cwd.to_string_lossy(), img.model, model_path);
    log::info!("model_vol: {}", model_vol);
    let extra_vols = vec![model_vol];

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

/*

curl --request POST \
    --url http://localhost:8787/completion \
    --header "Content-Type: application/json" \
    --data '{"prompt": "The national animals of the USA are","n_predict": 128}'

*/
