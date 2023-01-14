use super::*;
use crate::utils::{domain, exposed_ports, host_config, user, volume_string};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelayImage {
    pub name: String,
    pub version: String,
    pub node_env: String,
    pub port: String,
    pub links: Links,
}
impl RelayImage {
    pub fn new(name: &str, version: &str, node_env: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            node_env: node_env.to_string(),
            port: port.to_string(),
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

pub fn relay(
    project: &str,
    relay: &RelayImage,
    lnd: &lnd::LndImage,
    proxy: Option<proxy::ProxyImage>,
) -> Config<String> {
    // let img = "sphinx-relay";
    // let version = "latest";
    let img = "sphinxlightning/sphinx-relay";
    let version = relay.version.clone();
    let root_vol = "/relay/data";
    let mut conf = config::RelayConfig::new(&relay.name, &relay.port);
    conf.lnd(lnd);
    // add the LND volumes
    let lnd_vol = volume_string(project, &lnd.name, "/lnd");
    let mut extra_vols = vec![lnd_vol];
    let mut links = vec![domain(&lnd.name)];
    if let Some(p) = proxy {
        conf.proxy(&p);
        let proxy_vol = volume_string(project, &p.name, "/proxy");
        extra_vols.push(proxy_vol);
        links.push(domain(&p.name));
    }
    let mut relay_conf = config::relay_env_config(&conf);
    relay_conf.push(format!("NODE_ENV={}", &relay.node_env));
    Config {
        image: Some(format!("{}:{}", img, version)),
        hostname: Some(domain(&relay.name)),
        user: user(),
        exposed_ports: exposed_ports(vec![relay.port.clone()]),
        host_config: host_config(
            project,
            &relay.name,
            vec![relay.port.clone()],
            root_vol,
            Some(extra_vols),
            Some(links),
        ),
        env: Some(relay_conf),
        ..Default::default()
    }
}
