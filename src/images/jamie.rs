use super::*;
use crate::config::Node;
use crate::images::llama::LlamaImage;
use crate::images::mongo::MongoImage;
use crate::utils::{domain, exposed_ports, getenv, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct JamieImage {
    pub name: String,
    pub version: String,
    pub http_port: String,
    pub links: Links,
    pub host: Option<String>,
}

impl JamieImage {
    pub fn new(name: &str, version: &str) -> Self {
        // ports are hardcoded
        Self {
            name: name.to_string(),
            version: version.to_string(),
            http_port: "3000".to_string(),
            links: vec![],
            host: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("jamie.{}", h));
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for JamieImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let mongo = li.find_mongo().context("Chat: No Mongo")?;
        let llama = li.find_llama();
        Ok(jamie(self, &mongo, &llama))
    }
}

impl DockerHubImage for JamieImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "jamie".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

fn jamie(node: &JamieImage, mongo: &MongoImage, llama_opt: &Option<LlamaImage>) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let image = format!("{}/{}", repo.org, repo.repo);
    // let image = format!("ghcr.io/{}", image_end);

    let root_vol = &repo.root_volume;
    let ports = vec![node.http_port.clone()];

    let mut env = vec![
        format!(
            "MONGODB_URL=mongodb://{}:{}",
            domain(&mongo.name),
            mongo.http_port
        ),
        format!("PUBLIC_APP_NAME=SphinxChat"),
        format!("PUBLIC_APP_ASSETS=sphinx"),
        format!("PUBLIC_APP_COLOR=indigo"),
        format!("PUBLIC_APP_DESCRIPTION=Your Second Brain"),
    ];
    if let Ok(hf_token) = getenv("HF_TOKEN") {
        env.push(format!("HF_TOKEN={}", hf_token));
    }
    if let Some(llama) = llama_opt {
        let dotenvlocal = format!(
            r#"MODELS=`[
    {{
        "name": "Local Jamie",
        "preprompt": "",
        "parameters": {{
            "stop": ["<|end|>", "<|endoftext|>", "<|assistant|>"],
            "temperature": 0.7,
            "max_new_tokens": 1024,
            "truncate": 3071
        }},
        "endpoints": [{{
            "type" : "llamacpp",
            "baseURL": "http://{}:{}"
        }}],
    }},
]`"#,
            domain(&llama.name),
            llama.port
        );
        env.push(format!("DOTENV_LOCAL={}", dotenvlocal));
    }

    // let env = vec![format!(
    //     "MONGODB_URL=mongodb://{}:{}@{}:{}",
    //     mongo.user,
    //     mongo.pass,
    //     domain(&mongo.name),
    //     mongo.http_port
    // )];

    let c = Config {
        image: Some(format!("{}:{}", image, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        env: Some(env),
        host_config: host_config(&name, ports, root_vol, None, None),
        ..Default::default()
    };
    c
}

// CMD ["/bin/bash", "-c", "/app/entrypoint.sh"]
