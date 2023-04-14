use super::*;
use crate::config::{Clients, ExternalNodeType, Node};
use crate::conn::cln::setup as setup_cln;
use crate::conn::lnd::setup::test_mine_if_needed;
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
    pub async fn connect_client(
        &self,
        clients: &mut Clients,
        docker: &Docker,
        nodes: &Vec<Node>,
    ) -> Result<()> {
        sleep(1).await;
        let (client, test_mine_addy) = setup_cln(self, docker).await?;
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        if let Some(internal_btc) = li.find_btc() {
            test_mine_if_needed(test_mine_addy, &internal_btc.name, clients);
        }
        clients.cln.insert(self.name.clone(), client);
        Ok(())
    }
}

#[async_trait]
impl DockerConfig for ClnImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        if let Some(btc) = li.find_btc() {
            // internal BTC node
            let args = ClnBtcArgs::new(&domain(&btc.name), &btc.user, &btc.pass);
            Ok(cln(self, args))
        } else {
            // external BTC node
            let btcurl = nodes
                .iter()
                .find(|n| match n.as_external() {
                    Ok(i) => i.kind == ExternalNodeType::Btc,
                    Err(_) => false,
                })
                .context("CLN: no external BTC")?
                .as_external()?
                .url;
            let args = ClnBtcArgs::from_url(&btcurl)?;
            Ok(cln(self, args))
        }
    }
}

impl DockerHubImage for ClnImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "cln-sphinx".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ClnBtcArgs {
    rpcconnect: String,
    user: Option<String>,
    pass: Option<String>,
}
impl ClnBtcArgs {
    pub fn new(rpcconnect: &str, user: &Option<String>, pass: &Option<String>) -> Self {
        Self {
            rpcconnect: rpcconnect.to_string(),
            user: user.clone(),
            pass: pass.clone(),
        }
    }
    pub fn from_url(btcurl: &str) -> Result<Self> {
        let p = url::Url::parse(btcurl)?;
        let host = p.host().context("CLN: no host found in external BTC url")?;
        let fullhost = format!("{}", host);
        let username = if p.username() == "" {
            None
        } else {
            Some(p.username().to_string())
        };
        let password = if let Some(p) = p.password() {
            Some(p.to_string())
        } else {
            None
        };
        log::info!("CLN: connect to external BTC: {}", &fullhost);
        Ok(Self::new(&fullhost, &username, &password))
    }
}
fn cln(img: &ClnImage, btc: ClnBtcArgs) -> Config<String> {
    let mut ports = vec![img.peer_port.clone(), img.grpc_port.clone()];
    let root_vol = "/root/.lightning";
    let version = "0.1.0";
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

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
        format!("--bitcoin-rpcconnect={}", &btc.rpcconnect),
        "--bitcoin-rpcport=18443".to_string(),
        "--log-level=debug".to_string(),
        "--accept-htlc-tlv-types=133773310".to_string(),
    ];
    if let Some(u) = &btc.user {
        if let Some(p) = &btc.pass {
            cmd.push(format!("--bitcoin-rpcuser={}", u));
            cmd.push(format!("--bitcoin-rpcpassword={}", p));
        }
    }
    if img.plugins.contains(&ClnPlugin::HsmdBroker) {
        cmd.push(format!(
            "--subdaemon=hsmd:/usr/local/libexec/c-lightning/sphinx-key-broker"
        ));
        // docker run -it --entrypoint "/bin/bash" cln-sphinx
        // lightningd --version
        // let git_version = "2f1a063-modded";
        let git_version = "v23.02.2-50-gd15200c";
        environ.push(format!("GREENLIGHT_VERSION={}", git_version));
        if let Ok(pp) = img.peer_port.parse::<u16>() {
            if pp > 8876 {
                let mqtt_port = pp - 7852; // 1883
                environ.push(format!("BROKER_MQTT_PORT={}", mqtt_port));
                ports.push(mqtt_port.to_string());
                let http_port = pp - 1735 + 10; // 8010
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

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
