use super::traefik::{cln_traefik_labels, traefik_labels};
use super::*;
use crate::config::{Clients, ExternalNodeType, Node};
use crate::conn::cln::hsmd::HsmdClient;
use crate::conn::cln::setup as setup_cln;
use crate::conn::lnd::setup::test_mine_if_needed;
use crate::utils::{domain, exposed_ports, host_config};
use anyhow::{anyhow, Context, Result};
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
    pub host: Option<String>,
    pub git_version: Option<String>,
    pub frontend: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ClnPlugin {
    HsmdBroker,
    HtlcInterceptor,
}

pub struct ClnCreds {
    pub ca_cert: String,
    pub client_cert: String,
    pub client_key: String,
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
            host: None,
            git_version: None,
            frontend: None,
        }
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("{}.{}", self.name, h));
        }
    }
    pub fn plugins(&mut self, plugins: Vec<ClnPlugin>) {
        self.plugins = plugins
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub fn broker_frontend(&mut self) {
        self.frontend = Some(true);
    }
    pub fn remove_client(&self, clients: &mut Clients) {
        clients.cln.remove(&self.name);
    }
    pub async fn connect_client<Canceller>(
        &self,
        clients: &mut Clients,
        docker: &Docker,
        nodes: &Vec<Node>,
        canceller: Canceller,
    ) -> Result<()>
    where
        Canceller: Fn() -> bool,
    {
        sleep(1).await;
        let (client, test_mine_addy) = setup_cln(self, docker, canceller).await?;
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        if let Some(internal_btc) = li.find_btc() {
            test_mine_if_needed(test_mine_addy, &internal_btc.name, clients);
        }
        clients.cln.insert(self.name.clone(), client);
        if self.plugins.contains(&ClnPlugin::HtlcInterceptor) {
            match HsmdClient::new(self).await {
                Ok(client) => {
                    clients.hsmd.insert(self.name.clone(), client);
                }
                Err(e) => log::warn!("Hsmd client error: {:?}", e),
            };
        }
        Ok(())
    }
    pub fn credentials_paths(&self, root_dir: &str) -> ClnCreds {
        // let cln_root = format!("/{}/root/.lightning/{}", root_dir, &self.network);
        let cln_root = format!("/{}/{}", root_dir, &self.network);
        let ca_cert = format!("{}/ca.pem", cln_root);
        let client_cert = format!("{}/client.pem", cln_root);
        let client_key = format!("{}/client-key.pem", cln_root);
        ClnCreds {
            ca_cert,
            client_cert,
            client_key,
        }
    }
    pub async fn pre_startup(&self, _docker: &Docker) -> Result<()> {
        sleep(3).await;
        Ok(())
    }
}

#[async_trait]
impl DockerConfig for ClnImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let lss = li.find_lss();
        if let Some(btc) = li.find_btc() {
            // internal BTC node
            let args = ClnBtcArgs::new(
                &domain(&btc.name),
                &btc.user,
                &btc.pass,
                &None,
                &Some("http".to_string()),
            );
            Ok(cln(self, args, lss))
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
            Ok(cln(self, args, lss))
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
    port: Option<u16>,
    scheme: Option<String>,
}
impl ClnBtcArgs {
    pub fn new(
        rpcconnect: &str,
        user: &Option<String>,
        pass: &Option<String>,
        port: &Option<u16>,
        scheme: &Option<String>,
    ) -> Self {
        Self {
            rpcconnect: rpcconnect.to_string(),
            user: user.clone(),
            pass: pass.clone(),
            port: port.clone(),
            scheme: scheme.clone(),
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
        let port = p.port();
        let scheme = p.scheme();
        log::info!("CLN: connect to external BTC: {}", &fullhost);
        Ok(Self::new(
            &fullhost,
            &username,
            &password,
            &port,
            &Some(scheme.to_string()),
        ))
    }
    pub fn to_url(&self) -> String {
        let scheme = self.scheme.clone().unwrap_or("https".to_string());
        if let Some(u) = &self.user {
            if let Some(p) = &self.pass {
                return match self.port {
                    Some(port) => format!("{}://{}:{}@{}:{}", scheme, u, p, &self.rpcconnect, port),
                    None => format!("{}://{}:{}@{}", scheme, u, p, &self.rpcconnect),
                };
            }
        }
        match self.port {
            Some(port) => format!("{}://{}:{}", scheme, &self.rpcconnect, port),
            None => format!("{}://{}", scheme, &self.rpcconnect),
        }
    }
}

pub struct HsmdBrokerPorts {
    pub http_port: String,
    pub mqtt_port: String,
    pub ws_port: String,
}
pub fn hsmd_broker_ports(peer_port: &str) -> Result<HsmdBrokerPorts> {
    let pp = peer_port.parse::<u16>()?;
    if pp > 8876 {
        let mqtt_port = pp - 7852; // 1883
        let http_port = pp - 1735 + 10; // 8010
        let ws_port = pp - 1735 + 83; // 8083
        Ok(HsmdBrokerPorts {
            http_port: http_port.to_string(),
            mqtt_port: mqtt_port.to_string(),
            ws_port: ws_port.to_string(),
        })
    } else {
        Err(anyhow!("peer port too low"))
    }
}

fn cln(img: &ClnImage, btc: ClnBtcArgs, lss: Option<lss::LssImage>) -> Config<String> {
    let mut ports = vec![img.peer_port.clone(), img.grpc_port.clone()];
    let root_vol = "/root/.lightning";
    // let version = "0.2.3";
    let repo = img.repo();
    let image = format!("{}/{}", repo.org, repo.repo);

    let mut environ = vec![
        "EXPOSE_TCP=true".to_string(),
        format!("LIGHTNINGD_PORT={}", &img.peer_port),
        format!("LIGHTNINGD_NETWORK={}", &img.network),
    ];
    log::info!("CLN network: {}", &img.network);
    let mut alias = format!("sphinx-{}", &img.name);
    if let Some(host) = img.host.clone() {
        let parts: Vec<&str> = host.split(".").collect::<Vec<&str>>();
        // get the second one (cln.swarm13.sphinx.chat)
        if let Some(mid) = parts.get(1) {
            alias = format!("sphinx-{}-{}", &img.name, mid);
        }
    }
    let mut cmd = vec![
        format!("--alias={}", &alias),
        format!("--addr=0.0.0.0:{}", &img.peer_port),
        format!("--grpc-port={}", &img.grpc_port),
        format!("--network={}", &img.network),
        format!("--bitcoin-rpcconnect={}", &btc.rpcconnect),
        "--bitcoin-rpcport=18443".to_string(),
        "--log-level=info:gossipd".to_string(),
        "--log-level=info:channeld".to_string(),
        "--log-level=debug".to_string(),
        "--log-level=io:plugin-keysend".to_string(),
        "--accept-htlc-tlv-type=133773310".to_string(),
        "--database-upgrade=true".to_string(),
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
        // docker run -it --entrypoint "/bin/bash" sphinxlightning/cln-sphinx:latest
        // docker run -it --entrypoint "/bin/bash" cln-sphinx
        // lightningd --version
        // let git_version = "2f1a063-modded";
        let git_version = img
            .git_version
            .clone()
            .unwrap_or("v23.08-57-g420e0c9-modded".to_string());
        environ.push(format!("GREENLIGHT_VERSION={}", &git_version));
        // lss server (default to host.docker.internal)
        if let Some(lss) = lss {
            let vls_lss = format!("http://{}:{}", &domain(&lss.name), &lss.port);
            log::info!("hook up to LSS {}", &vls_lss);
            environ.push(format!("VLS_LSS={}", &vls_lss));
        }
        // if let Ok(lssurl) = std::env::var("LSS_URL") {
        //     if lssurl.len() > 0 {
        //         vls_lss = lssurl;
        //     }
        // }

        if let Ok(hbp) = hsmd_broker_ports(&img.peer_port) {
            environ.push(format!("BROKER_MQTT_PORT={}", &hbp.mqtt_port));
            ports.push(hbp.mqtt_port);
            environ.push(format!("BROKER_HTTP_PORT={}", &hbp.http_port));
            ports.push(hbp.http_port);
            environ.push(format!("BROKER_WS_PORT={}", &hbp.ws_port));
            ports.push(hbp.ws_port);
        }
        environ.push(format!("BROKER_NETWORK={}", img.network));

        if img.frontend.unwrap_or(false) {
            let rpc_url = btc.to_url();
            log::info!("CLN BITCOIND_RPC_URL {}", &rpc_url);
            environ.push(format!("BITCOIND_RPC_URL={}", rpc_url));
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
    };
    let mut c = Config {
        image: Some(format!("{}:{}", image, img.version)),
        // image: Some("cln-sphinx:latest".to_string()),
        hostname: Some(domain(&img.name)),
        domainname: Some(img.name.clone()),
        cmd: Some(cmd),
        exposed_ports: exposed_ports(ports.clone()),
        env: Some(environ),
        host_config: host_config(&img.name, ports, root_vol, None),
        ..Default::default()
    };
    if let Some(host) = img.host.clone() {
        if img.plugins.contains(&ClnPlugin::HsmdBroker) {
            if let Ok(hbp) = hsmd_broker_ports(&img.peer_port) {
                c.labels = Some(cln_traefik_labels(
                    &img.name,
                    &host,
                    &img.peer_port,
                    &hbp.http_port,
                    &hbp.mqtt_port,
                ))
            }
        } else {
            c.labels = Some(traefik_labels(&img.name, &host, &img.peer_port, false));
        }
    }
    c
}

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
