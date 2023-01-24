use super::*;
use crate::utils::{domain, exposed_ports, host_config, volume_string};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

// in relay:
// docker build --no-cache -f Dockerfile.swarm .

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
impl DockerHubImage for RelayImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-relay-swarm".to_string(),
        }
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
    let repo = relay.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let version = relay.version.clone();
    let root_vol = "/relay";
    let mut conf = RelayConfig::new(&relay.name, &relay.port);
    conf.lnd(lnd);
    // add the LND volumes
    let lnd_vol = volume_string(project, &lnd.name, "/lnd");
    let mut extra_vols = vec![lnd_vol];
    let mut links = vec![domain(&lnd.name)];
    // add the optional Proxy stuff
    if let Some(p) = proxy {
        conf.proxy(&p);
        let proxy_vol = volume_string(project, &p.name, "/proxy");
        extra_vols.push(proxy_vol);
        links.push(domain(&p.name));
    }
    // relay config from env
    let mut relay_conf = relay_env_config(&conf);
    relay_conf.push(format!("NODE_ENV={}", &relay.node_env));
    Config {
        image: Some(format!("{}:{}", img, version)),
        hostname: Some(domain(&relay.name)),
        // user: Some(format!("1000")), // user(),
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

// #[serde(skip_serializing_if = "Option::is_none")]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct RelayConfig {
    pub node_ip: String,
    pub lnd_ip: String,
    pub lnd_port: String,
    pub public_url: String,
    pub tls_location: String,
    pub macaroon_location: String,
    pub node_http_port: String,
    pub tribes_mqtt_port: String,
    pub db_dialect: String,
    pub db_storage: String,
    pub tribes_insecure: Option<String>,
    pub node_http_protocol: Option<String>,
    pub transport_private_key_location: Option<String>,
    pub transport_public_key_location: Option<String>,
    pub proxy_macaroons_dir: Option<String>,
    pub proxy_tls_location: Option<String>,
    pub proxy_lnd_ip: Option<String>,
    pub proxy_lnd_port: Option<String>,
    pub proxy_admin_token: Option<String>,
    pub proxy_admin_url: Option<String>,
    pub proxy_new_nodes: Option<String>,
    pub proxy_initial_sats: Option<String>,
}

impl RelayConfig {
    pub fn new(_name: &str, port: &str) -> Self {
        Self {
            node_http_port: port.to_string(),
            public_url: format!("127.0.0.1:{}", port).to_string(),
            ..Default::default()
        }
    }
    pub fn lnd(&mut self, lnd: &lnd::LndImage) {
        self.lnd_ip = domain(&lnd.name);
        self.lnd_port = lnd.rpc_port.to_string();
        self.tls_location = "/lnd/tls.cert".to_string();
        self.macaroon_location = format!("/lnd/data/chain/bitcoin/{}/admin.macaroon", lnd.network);
    }
    pub fn proxy(&mut self, proxy: &proxy::ProxyImage) {
        self.proxy_lnd_ip = Some(domain(&proxy.name));
        self.proxy_lnd_port = Some(proxy.port.clone());
        self.proxy_admin_token = proxy.admin_token.clone();
        self.proxy_macaroons_dir = Some("/proxy/macaroons".to_string());
        self.proxy_tls_location = Some("/proxy/tls.cert".to_string());
        self.proxy_admin_url = Some(format!("{}:{}", domain(&proxy.name), proxy.admin_port));
        self.proxy_new_nodes = proxy.new_nodes.clone();
    }
}

type JsonMap = std::collections::HashMap<String, String>;

pub fn relay_env_config(c: &RelayConfig) -> Vec<String> {
    let blah = serde_json::to_value(&c).unwrap();
    let conf: JsonMap = serde_json::from_value(blah).unwrap();
    let mut ret = Vec::new();
    for (k, v) in conf.iter() {
        ret.push(format!("{}={}", k, v));
    }
    ret
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            node_ip: "127.0.0.1".to_string(),
            lnd_ip: "lnd.sphinx".to_string(),
            lnd_port: "10009".to_string(),
            public_url: "127.0.0.0:3000".to_string(),
            tls_location: "/relay/.lnd/tls.cert".to_string(),
            macaroon_location: "/relay/.lnd/data/chain/bitcoin/regtest/admin.macaroon".to_string(),
            node_http_port: "3000".to_string(),
            tribes_mqtt_port: "1883".to_string(),
            db_dialect: "sqlite".to_string(),
            db_storage: "/relay/sphinx.db".to_string(),
            node_http_protocol: None,
            tribes_insecure: None,
            transport_private_key_location: None,
            transport_public_key_location: None,
            proxy_macaroons_dir: None,
            proxy_tls_location: None,
            proxy_lnd_ip: None,
            proxy_lnd_port: None,
            proxy_admin_token: None,
            proxy_admin_url: None,
            proxy_new_nodes: None,
            proxy_initial_sats: None,
        }
    }
}

// using env instead of file
pub fn _relay_config(project: &str, name: &str) -> RelayConfig {
    use std::fs;
    use std::io::Write;
    // not using vol/ anymore...
    let path = format!("vol/{}/{}.json", project, name);
    match fs::read(path.clone()) {
        Ok(data) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => Default::default(),
        },
        Err(_e) => {
            let st = serde_json::to_string_pretty::<RelayConfig>(&Default::default())
                .expect("failed to make json string");
            let mut file = fs::File::create(path).expect("create failed");
            file.write_all(st.as_bytes()).expect("write failed");
            Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_relay_config() {
        let mut c = RelayConfig::new("relay", "3000");
        c.lnd(&lnd::LndImage::new(
            "lnd",
            "v0.14.3-beta.rc1",
            "regtest",
            "10009",
            "9735",
        ));
        relay_env_config(&c);
        assert!(true == true)
    }
}

// pub async fn get_conf() -> &'static Config {
//     let conf = CONFIG.lock().await;
//     &conf
// }
