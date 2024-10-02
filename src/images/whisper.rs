use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::utils::{add_gpus_to_host_config, domain, exposed_ports, host_config};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct WhisperImage {
    pub name: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
}

const VERSION: &str = "latest-cuda";

impl WhisperImage {
    pub fn new(name: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            port: port.to_string(),
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
impl DockerConfig for WhisperImage {
    async fn make_config(&self, _nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        // let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        Ok(whisper(self)?)
    }
}

impl DockerHubImage for WhisperImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "fedirz".to_string(),
            repo: "faster-whisper-server".to_string(),
            root_volume: "/home/whisper".to_string(),
        }
    }
}

fn whisper(img: &WhisperImage) -> Result<Config<String>> {
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let huggingface = "/home/admin/.cache/huggingface";
    let extra_vols = vec![format!("{huggingface}:/root/.cache/huggingface")];

    let env = vec![format!("UVICORN_PORT={}", img.port)];

    let mut hc = host_config(&img.name, ports.clone(), root_vol, Some(extra_vols), None).unwrap();
    add_gpus_to_host_config(&mut hc, 1);
    let mut c = Config {
        image: Some(format!("{}:{}", image, VERSION)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: Some(hc),
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
    --url http://localhost:8989/completion \
    --header "Content-Type: application/json"

export WHISPER__MODEL=Systran/faster-whisper-tiny.en

docker run --publish 8000:8000 --volume ~/.cache/huggingface:/root/.cache/huggingface --env WHISPER__MODEL=$WHISPER__MODEL fedirz/faster-whisper-server:latest-cpu

convert to pcm:
ffmpeg -y -hide_banner -loglevel quiet -i talk.mp4 -ac 1 -ar 16000 -f s16le -acodec pcm_s16le audio.pcm

stream to server:
cat audio.pcm | pv -qL 32000 | websocat --no-close --binary 'ws://localhost:8000/v1/audio/transcriptions?language=en'
*/
