use super::traefik::{navfiber_boltwall_shared_host, traefik_labels};
use super::*;
use crate::config::Node;
use crate::conn::lnd::utils::{dl_cert_to_base64, dl_macaroon};
use crate::images::lnd::to_lnd_network;
use crate::secrets;
use crate::utils::{domain, exposed_ports, host_config, volume_string};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BoltwallImage {
    pub name: String,
    pub version: String,
    pub port: String,
    pub host: Option<String>,
    pub session_secret: String,
    pub external_lnd: Option<ExternalLnd>,
    pub links: Links,
    pub admin_token: Option<String>,
}

impl BoltwallImage {
    pub fn new(name: &str, version: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            port: port.to_string(),
            host: None,
            session_secret: secrets::random_word(32),
            external_lnd: None,
            links: vec![],
            admin_token: Some(secrets::random_word(32)),
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links);
    }
    pub fn external_lnd(&mut self, external_lnd: ExternalLnd) {
        self.external_lnd = Some(external_lnd);
    }
    pub fn set_admin_token(&mut self, at: &str) {
        self.admin_token = Some(at.to_string());
    }
    pub fn host(&mut self, eh: Option<String>) {
        if let Some(h) = eh {
            self.host = Some(format!("boltwall.{}", h));
        }
        // boltwall host is on the vanity address /api
        if let Some(sh) = navfiber_boltwall_shared_host() {
            self.host = Some(format!("{}/api", sh))
        }
    }
}

#[async_trait]
impl DockerConfig for BoltwallImage {
    async fn make_config(&self, nodes: &Vec<Node>, docker: &Docker) -> Result<Config<String>> {
        let li = LinkedImages::from_nodes(self.links.clone(), nodes);

        let jarvis_node = li.find_jarvis().context("Boltwall: No Jarvis")?;

        if let Some(ext) = self.external_lnd.clone() {
            return Ok(boltwall(
                &self,
                None,
                Some(ext.creds),
                None,
                &jarvis_node,
                Some(ext.address),
            ));
        }

        let lnd_node = li.find_lnd();
        let mut lnd_creds = None;
        if let Some(lnd) = &lnd_node {
            let cert_path = "/home/.lnd/tls.cert";
            let cert64 = dl_cert_to_base64(docker, &lnd.name, cert_path).await?;
            // let cert64 = strip_pem_prefix_suffix(&cert_full);
            let netwk = to_lnd_network(lnd.network.as_str());
            let macpath = format!("/home/.lnd/data/chain/bitcoin/{}/admin.macaroon", netwk);
            let mac = dl_macaroon(docker, &lnd.name, &macpath).await?;
            lnd_creds = Some(LndCreds {
                macaroon: mac.to_string(),
                cert: cert64.to_string(),
            });
        }
        let cln_node = li.find_cln();

        Ok(boltwall(
            &self,
            lnd_node,
            lnd_creds,
            cln_node,
            &jarvis_node,
            None,
        ))
    }
}

impl DockerHubImage for BoltwallImage {
    fn repo(&self) -> Repository {
        Repository {
            org: "sphinxlightning".to_string(),
            repo: "sphinx-boltwall".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ExternalLnd {
    pub address: String,
    pub creds: LndCreds,
}

impl ExternalLnd {
    pub fn new(address: &str, mac: &str, cert: &str) -> Self {
        Self {
            address: address.to_string(),
            creds: LndCreds {
                macaroon: mac.to_string(),
                cert: cert.to_string(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct LndCreds {
    macaroon: String,
    cert: String,
}

fn boltwall(
    node: &BoltwallImage,
    lnd_node: Option<lnd::LndImage>,
    lnd_creds: Option<LndCreds>,
    cln_node: Option<cln::ClnImage>,
    jarvis: &jarvis::JarvisImage,
    external_lnd_address: Option<String>,
) -> Config<String> {
    let name = node.name.clone();
    let repo = node.repo();
    let img = format!("{}/{}", repo.org, repo.repo);
    let ports = vec![node.port.clone()];
    let root_vol = "/boltwall";

    let mut env = vec![
        format!("PORT={}", node.port),
        format!("BOLTWALL_MIN_AMOUNT=2"),
        format!("LIQUID_SERVER=https://liquid.sphinx.chat/"),
        format!(
            "JARVIS_BACKEND_URL=http://{}:{}",
            domain(&jarvis.name),
            jarvis.port
        ),
        format!("SESSION_SECRET={}", node.session_secret),
        format!("NODE_ENV=swarm"),
    ];
    let mut extra_vols = None;

    // EXTERNAL LND
    if let Some(ext_addy) = external_lnd_address {
        env.push(format!("LND_SOCKET={}", ext_addy));
        if let Some(creds) = lnd_creds {
            env.push(format!("LND_TLS_CERT={}", &creds.cert));
            env.push(format!("LND_MACAROON={}", &creds.macaroon));
        }
    // INTERNAL LND
    } else if let Some(lnd_node) = lnd_node {
        let lnd_socket = format!("{}:{}", &domain(&lnd_node.name), lnd_node.rpc_port);
        env.push(format!("LND_SOCKET={}", lnd_socket));
        if let Some(creds) = lnd_creds {
            env.push(format!("LND_TLS_CERT={}", &creds.cert));
            env.push(format!("LND_MACAROON={}", &creds.macaroon));
        }
    // INTERNAL CLN
    } else if let Some(cln) = cln_node {
        let cln_vol = volume_string(&cln.name, "/cln");
        extra_vols = Some(vec![cln_vol]);
        let creds = cln.credentials_paths("cln");
        env.push(format!("CLN_TLS_LOCATION={}", creds.ca_cert));
        env.push(format!("CLN_TLS_KEY_LOCATION={}", creds.client_key));
        env.push(format!("CLN_TLS_CHAIN_LOCATION={}", creds.client_cert));
        env.push(format!("CLN_URI={}:{}", domain(&cln.name), cln.grpc_port));
    }
    // the webhook url "callback"
    if let Some(h) = &node.host {
        env.push(format!("HOST_URL=https://{}", h));
    }
    // admin token for setting admin pubkey
    if let Some(at) = &node.admin_token {
        env.push(format!("ADMIN_TOKEN={}", at));
    }
    let mut c = Config {
        image: Some(format!("{}:{}", img, node.version)),
        hostname: Some(domain(&name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(&name, ports, root_vol, extra_vols, None),
        env: Some(env),
        ..Default::default()
    };
    if let Some(host) = node.host.clone() {
        // production tls extra domain
        c.labels = Some(traefik_labels(&node.name, &host, &node.port, false));
    }
    c
}
