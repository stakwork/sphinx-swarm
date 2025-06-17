use super::traefik::traefik_labels;
use super::*;
use crate::config::Node;
use crate::images::dufs::DufsImage;
use crate::utils::{domain, exposed_ports, host_config, volume_string};
use anyhow::Result;
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct RqbitImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub links: Links,
}

impl RqbitImage {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: "3030".to_string(), // need to figure out config here
            host: None,
            links: vec![],
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
impl DockerConfig for RqbitImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let dufs = li.find_dufs();
        Ok(rqbit(self, &dufs)?)
    }
}

impl DockerHubImage for RqbitImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "sphinxlightning".to_string(),
            repo: "rqbit".to_string(),
            root_volume: "/home/.rqbit".to_string(),
        }
    }
}

fn rqbit(img: &RqbitImage, dufs_opt: &Option<DufsImage>) -> Result<Config<String>> {
    let repo = img.repo();
    let image = img.image();

    let root_vol = &repo.root_volume;

    let ports = vec![img.port.clone()];

    let (extra_vols, files_dir) = if let Some(dufs) = dufs_opt {
        (
            Some(vec![volume_string(&dufs.name, &dufs.files_dir)]),
            dufs.files_dir.clone(),
        )
    } else {
        (None, "/files".to_string())
    };

    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        hostname: Some(domain(&img.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&img.name, ports, root_vol, extra_vols, None),
        env: None,
        entrypoint: Some(vec![
            "/usr/src/rqbit".to_string(),
            "server".to_string(),
            "start".to_string(),
            files_dir,
        ]),
        ..Default::default()
    };
    if let Some(host) = &img.host {
        c.labels = Some(traefik_labels(&img.name, &host, &img.port, false))
    }
    Ok(c)
}
