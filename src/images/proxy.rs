use super::*;
use crate::config::{Clients, Node};
use crate::conn::proxy::ProxyAPI;
use crate::images::lnd::to_lnd_network;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config, volume_string};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bollard::{container::Config, Docker};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ProxyImage {
    pub name: String,
    pub version: String,
    pub network: String,
    pub port: String,
    pub admin_port: String,
    pub admin_token: Option<String>,
    pub store_key: Option<String>,
    pub new_nodes: Option<String>, // for relay
    pub links: Links,
}

impl ProxyImage {
    pub fn new(name: &str, version: &str, network: &str, port: &str, admin_port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            port: port.to_string(),
            admin_port: admin_port.to_string(),
            admin_token: Some(secrets::random_word(12)),
            store_key: Some(secrets::hex_secret()),
            new_nodes: None,
            links: vec![],
        }
    }
    pub fn new_nodes(&mut self, new_nodes: Option<String>) {
        self.new_nodes = new_nodes;
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
    pub async fn connect_client(&self, clients: &mut Clients) -> Result<()> {
        match ProxyAPI::new(self).await {
            Ok(client) => {
                clients.proxy.insert(self.name.clone(), client);
            }
            Err(e) => log::warn!("ProxyAPI error: {:?}", e),
        };
        sleep(1).await;
        Ok(())
    }
}

#[async_trait]
impl DockerConfig for ProxyImage {
    async fn make_config(&self, nodes: &Vec<Node>, _docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);
        let lnd = li.find_lnd();
        let mut cln = li.find_cln();
        if let None = lnd {
            if let None = cln {
                return Err(anyhow!("LND or CLN required for Proxy"));
            }
        } else {
            cln = None
        }
        Ok(proxy(&self, lnd, cln))
    }
}

impl DockerHubImage for ProxyImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-proxy".to_string(),
        }
    }
}

fn proxy(
    proxy: &ProxyImage,
    lnd: Option<lnd::LndImage>,
    cln: Option<cln::ClnImage>,
) -> Config<String> {
    let repo = proxy.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let version = proxy.version.clone();
    // let img = "sphinx-proxy";
    // let version = "latest";
    let root_vol = "/app/proxy";
    let ports = vec![proxy.port.clone(), proxy.admin_port.clone()];

    // lnd or proxy uses "mainnet" instead of "bitcoin"
    let lnd_netwk = to_lnd_network(proxy.network.as_str()).to_string();

    let mut mode = "lnd".to_string();
    let mut extra_vols = vec![];
    let mut rpc_port = "10009".to_string();
    let mut thename = "127.0.0.1".to_string();
    let mut extra_cmd = vec![];
    if let Some(lnd) = lnd {
        let lnd_vol = volume_string(&lnd.name, "/lnd");
        extra_vols.push(lnd_vol);
        rpc_port = lnd.rpc_port;
        thename = lnd.name;
        let macpath = format!("/lnd/data/chain/bitcoin/{}/admin.macaroon", &lnd_netwk);
        extra_cmd.push(format!("--macaroon-location={}", macpath))
    } else if let Some(cln) = cln {
        mode = "cln".to_string();
        let cln_vol = volume_string(&cln.name, "/cln");
        extra_vols.push(cln_vol);
        rpc_port = cln.grpc_port.clone();
        thename = cln.name.clone();
        let creds = cln.credentials_paths();
        // cln uses "bitcoin" not "mainnet" network name
        extra_cmd.push(format!("--cln-ca-cert={}", creds.ca_cert));
        extra_cmd.push(format!("--cln-client-cert={}", creds.client_cert));
        extra_cmd.push(format!("--cln-client-key={}", creds.client_key));
    }

    let mut cmd = vec![
        "/app/sphinx-proxy".to_string(),
        format!("--mode={}", mode),
        "--rpclisten=0.0.0.0:11111".to_string(),
        "--store-dir=/app/proxy/badger".to_string(),
        "--bitcoin.active".to_string(),
        "--bitcoin.basefee=0".to_string(),
        "--use-hd-keys".to_string(), // HD key derivation
        format!("--bitcoin.{}", &lnd_netwk),
        format!("--rpclisten=0.0.0.0:{}", &proxy.port),
        format!("--admin-port={}", &proxy.admin_port),
        format!("--lnd-ip={}.sphinx", &thename),
        format!("--lnd-port={}", &rpc_port),
        format!("--tlsextradomain={}.sphinx", proxy.name),
        "--tlscertpath=/app/proxy/tls.cert".to_string(),
        "--tlskeypath=/app/proxy/tls.key".to_string(),
        "--tls-location=/lnd/tls.cert".to_string(),
        "--unlock-pwd=hi123456".to_string(),
        "--server-macaroons-dir=/app/proxy/macaroons".to_string(),
        "--channels-start=1".to_string(),
        "--initial-msat=0".to_string(),
    ];
    if let Some(at) = &proxy.admin_token {
        cmd.push(format!("--admin-token={}", &at));
    }
    if let Some(sk) = &proxy.store_key {
        cmd.push(format!("--store-key={}", &sk));
    }
    // add in extra cmds from lnd/cln
    extra_cmd.iter().for_each(|c| {
        cmd.push(c.to_string());
    });
    Config {
        image: Some(format!("{}:{}", img, version)),
        hostname: Some(domain(&proxy.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&proxy.name, ports, root_vol, Some(extra_vols)),
        cmd: Some(cmd),
        ..Default::default()
    }
}

async fn sleep(n: u64) {
    rocket::tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
