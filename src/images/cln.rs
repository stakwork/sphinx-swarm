use super::*;
use crate::config::Node;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ClnImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub peer_port: String,
    pub grpc_port: String,
    pub plugins: Vec<ClnPlugin>,
    pub links: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ClnPlugin {
    HsmdBroker,
    HtlcInterceptor,
}

impl ClnImage {
    pub fn new(name: &str, version: &str, network: &str, peer_port: &str, grpc_port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            peer_port: peer_port.to_string(),
            grpc_port: grpc_port.to_string(),
            plugins: vec![],
            links: vec![],
        }
    }
    pub fn plugins(&mut self, plugins: Vec<ClnPlugin>) {
        self.plugins = plugins
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}

#[async_trait]
impl DockerConfig for ClnImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let btc = li.find_btc().context("BTC required for CLN")?;
        Ok(cln(self, &btc))
    }
}

impl DockerHubImage for ClnImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "elementsproject".to_string(),
            repo: "lightningd".to_string(),
        }
    }
}

pub fn cln(img: &ClnImage, btc: &btc::BtcImage) -> Config<String> {
    let mut ports = vec![img.peer_port.clone(), img.grpc_port.clone()];
    let root_vol = "/root/.lightning";
    // let version = "v22.11.1";
    // let repo = img.repo();
    // let image = format!("{}/{}", repo.org, repo.repo);

    let mut environ = vec![
        "EXPOSE_TCP=true".to_string(),
        format!("LIGHTNINGD_PORT={}", &img.peer_port),
        format!("LIGHTNINGD_NETWORK={}", &img.network),
    ];
    let mut cmd = vec![
        format!("--alias=sphinx-{}", &img.name),
        format!("--addr=0.0.0.0:{}", &img.peer_port),
        format!("--grpc-port={}", &img.grpc_port),
        format!("--network={}", &img.network),
        format!("--bitcoin-rpcconnect={}", &domain(&btc.name)),
        "--bitcoin-rpcport=18443".to_string(),
        format!("--bitcoin-rpcuser={}", btc.user),
        format!("--bitcoin-rpcpassword={}", btc.pass),
        "--log-level=debug".to_string(),
        "--accept-htlc-tlv-types=133773310".to_string(),
    ];
    if img.plugins.contains(&ClnPlugin::HsmdBroker) {
        cmd.push(format!(
            "--subdaemon=hsmd:/usr/local/libexec/c-lightning/sphinx-key-broker"
        ));
        // docker run -it --entrypoint "/bin/bash" cln-sphinx
        // lightningd --version
        let git_version = "280b49a-modded";
        environ.push(format!("GREENLIGHT_VERSION={}", git_version));
        if let Ok(pp) = img.peer_port.parse::<u16>() {
            if pp > 8876 {
                let mqtt_port = pp - 7852; // 1883
                environ.push(format!("BROKER_MQTT_PORT={}", mqtt_port));
                ports.push(mqtt_port.to_string());
                let http_port = pp - 1735; // 8000
                environ.push(format!("BROKER_HTTP_PORT={}", http_port));
                ports.push(http_port.to_string());
            }
        }
    }
    // add the interceptor at grpc port + 200
    if img.plugins.contains(&ClnPlugin::HtlcInterceptor) {
        cmd.push(format!(
            "--plugin=/usr/local/libexec/c-lightning/plugins/gateway-cln-extension"
        ));
        if let Ok(rp) = img.grpc_port.parse::<u16>() {
            let plugin_port = rp + 200;
            environ.push(format!(
                "FM_CLN_EXTENSION_LISTEN_ADDRESS=0.0.0.0:{}",
                plugin_port.to_string()
            ));
            ports.push(plugin_port.to_string());
        }
    }
    Config {
        // image: Some(format!("{}:{}", image, version)),
        image: Some("cln-sphinx:latest".to_string()),
        hostname: Some(domain(&img.name)),
        domainname: Some(img.name.clone()),
        cmd: Some(cmd),
        exposed_ports: exposed_ports(ports.clone()),
        env: Some(environ),
        host_config: host_config(&img.name, ports, root_vol, None),
        ..Default::default()
    }
}
