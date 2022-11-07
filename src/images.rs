use crate::config;
use crate::utils::{domain, exposed_ports, files_volume, host_config, volume_string};
use bollard::container::Config;
use serde::{Deserialize, Serialize};

// ports are tcp
// volumes are mapped to {PWD}/vol/{name}:

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Image {
    Btc(BtcImage),
    Lnd(LndImage),
    Relay(RelayImage),
    Proxy(ProxyImage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BtcImage {
    pub name: String,
    pub network: String,
    pub user: String,
    pub pass: String,
}
impl BtcImage {
    pub fn new(name: &str, network: &str, user: &str, pass: &str) -> Self {
        Self {
            name: name.to_string(),
            network: network.to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LndImage {
    pub name: String,
    pub network: String,
    pub port: String,
    pub http_port: Option<String>,
}
impl LndImage {
    pub fn new(name: &str, network: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            network: network.to_string(),
            port: port.to_string(),
            http_port: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelayImage {
    pub name: String,
    pub port: String,
}
impl RelayImage {
    pub fn new(name: &str, port: &str) -> Self {
        Self {
            name: name.to_string(),
            port: port.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyImage {
    pub name: String,
    pub network: String,
    pub port: String,
    pub admin_port: String,
    pub admin_token: String,
    pub store_key: String,
    pub new_nodes: Option<String>, // for relay
}

impl ProxyImage {
    pub fn new(
        name: &str,
        network: &str,
        port: &str,
        admin_port: &str,
        admin_token: &str,
        store_key: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            network: network.to_string(),
            port: port.to_string(),
            admin_port: admin_port.to_string(),
            admin_token: admin_token.to_string(),
            store_key: store_key.to_string(),
            new_nodes: None,
        }
    }
    pub fn new_nodes(&mut self, new_nodes: Option<String>) {
        self.new_nodes = new_nodes;
    }
}

pub fn lnd(project: &str, lnd: &LndImage, btc: &BtcImage) -> Config<String> {
    let network = match lnd.network.as_str() {
        "bitcoin" => "mainnet",
        "simnet" => "simnet",
        "regtest" => "regtest",
        _ => "regtest",
    };
    let version = "v0.14.3-beta.rc1".to_string();
    let peering_port = "9735";
    let mut ports = vec![peering_port.to_string(), lnd.port.clone()];
    let root_vol = "/root/.lnd";
    let links = Some(vec![domain(&btc.name)]);
    let mut cmd = vec![
        format!("--bitcoin.{}", network).to_string(),
        format!("--rpclisten=0.0.0.0:{}", &lnd.port).to_string(),
        format!("--tlsextradomain={}.sphinx", lnd.name).to_string(),
        format!("--alias={}", &lnd.name).to_string(),
        format!("--bitcoind.rpcuser={}", &btc.user).to_string(),
        format!("--bitcoind.rpcpass={}", &btc.pass).to_string(),
        format!("--bitcoind.rpchost={}.sphinx", &btc.name).to_string(),
        format!("--bitcoind.zmqpubrawblock=tcp://{}.sphinx:28332", &btc.name).to_string(),
        format!("--bitcoind.zmqpubrawtx=tcp://{}.sphinx:28333", &btc.name).to_string(),
        "--debuglevel=info".to_string(),
        "--accept-keysend".to_string(),
        "--bitcoin.active".to_string(),
        "--bitcoin.node=bitcoind".to_string(),
        "--bitcoin.defaultchanconfs=2".to_string(),
    ];
    if let Some(hp) = lnd.http_port.clone() {
        ports.push(hp.clone());
        let rest_host = "0.0.0.0";
        cmd.push(format!("--restlisten={}:{}", rest_host, hp).to_string());
    }
    Config {
        image: Some(format!("lightninglabs/lnd:{}", version).to_string()),
        hostname: Some(domain(&lnd.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(project, &lnd.name, ports, root_vol, None, links),
        cmd: Some(cmd),
        ..Default::default()
    }
}

pub fn postgres(project: &str) -> Config<String> {
    let name = "postgres";
    let root_vol = "/var/lib/postgresql/data";
    Config {
        image: Some("postgres".to_string()),
        hostname: Some(domain(name)),
        host_config: host_config(project, name, vec![], root_vol, None, None),
        ..Default::default()
    }
}

pub fn relay(
    project: &str,
    relay: &RelayImage,
    lnd: &LndImage,
    proxy: Option<&ProxyImage>,
) -> Config<String> {
    let img = "sphinx-relay";
    let version = "latest";
    // let img = "sphinxlightning/sphinx-relay";
    // let version = "v2.2.10".to_string();
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
    Config {
        image: Some(format!("{}:{}", img, version)),
        hostname: Some(domain(&relay.name)),
        exposed_ports: exposed_ports(vec![relay.port.clone()]),
        host_config: host_config(
            project,
            &relay.name,
            vec![relay.port.clone()],
            root_vol,
            Some(extra_vols),
            Some(links),
        ),
        env: Some(config::relay_env_config(&conf)),
        ..Default::default()
    }
}

pub fn proxy(project: &str, proxy: &ProxyImage, lnd: &LndImage) -> Config<String> {
    let img = "sphinxlightning/sphinx-proxy";
    let version = "0.1.2".to_string();
    // let img = "sphinx-proxy";
    // let version = "latest";
    let macpath = format!(
        "--macaroon-location=/lnd/data/chain/bitcoin/{}/admin.macaroon",
        proxy.network
    );
    let links = vec![domain(&lnd.name)];
    // let vols = vec!["/cert", "/badger", "/macaroons"];
    let root_vol = "/proxy";
    let badger_vol = volume_string(project, &format!("{}/badger", &proxy.name), "/badger");
    let mut extra_vols = vec![badger_vol];
    let lnd_vol = volume_string(project, &lnd.name, "/lnd");
    extra_vols.push(lnd_vol);
    extra_vols.push(files_volume());
    let ports = vec![proxy.port.clone(), proxy.admin_port.clone()];
    Config {
        image: Some(format!("{}:{}", img, version)),
        hostname: Some(domain(&proxy.name)),
        exposed_ports: exposed_ports(ports.clone()),
        host_config: host_config(
            project,
            &proxy.name,
            ports,
            root_vol,
            Some(extra_vols),
            Some(links),
        ),
        cmd: Some(vec![
            "/app/sphinx-proxy".to_string(),
            "--configfile=/files/lnd_proxy.conf".to_string(),
            macpath.to_string(),
            "--bitcoin.active".to_string(),
            "--bitcoin.basefee=0".to_string(),
            format!("--bitcoin.{}", &proxy.network),
            format!("--rpclisten=0.0.0.0:{}", &proxy.port),
            format!("--store-key={}", &proxy.store_key),
            format!("--admin-token={}", &proxy.admin_token),
            format!("--admin-port={}", &proxy.admin_port),
            format!("--lnd-ip={}.sphinx", &lnd.name),
            format!("--lnd-port={}", &lnd.port),
            format!("--tlsextradomain={}.sphinx", proxy.name),
            "--tlscertpath=/proxy/tls.cert".to_string(),
            "--tlskeypath=/proxy/tls.key".to_string(),
            "--tls-location=/lnd/tls.cert".to_string(),
            "--unlock-pwd=hi123456".to_string(),
            "--server-macaroons-dir=/proxy/macaroons".to_string(),
            "--channels-start=2".to_string(),
            "--initial-msat=500000".to_string(),
        ]),
        ..Default::default()
    }
}

pub fn btc(project: &str, node: &BtcImage) -> Config<String> {
    let btc_version = "23.0";
    let ports = vec![
        "18443".to_string(),
        "28332".to_string(),
        "28333".to_string(),
    ];
    let root_vol = "/home/bitcoin/.bitcoin";
    // let vols = vec!["/home/bitcoin/.bitcoin"];
    Config {
        image: Some(format!("ruimarinho/bitcoin-core:{}", btc_version)),
        hostname: Some(domain(&node.name)),
        cmd: Some(vec![
            format!("-{}=1", node.network),
            format!("-rpcuser={}", node.user),
            format!("-rpcpassword={}", node.pass),
            format!("-rpcbind={}.sphinx", node.name),
            "-rpcallowip=0.0.0.0/0".to_string(),
            "-rpcbind=0.0.0.0".to_string(),
            "-rpcport=18443".to_string(),
            "-server".to_string(),
            "-rpcallowip=0.0.0.0/0".to_string(),
            "-fallbackfee=0.0002".to_string(),
            "-zmqpubhashblock=tcp://0.0.0.0:28332".to_string(),
            "-zmqpubhashtx=tcp://0.0.0.0:28333".to_string(),
            "-rpcbind=127.0.0.1".to_string(),
        ]),
        host_config: host_config(project, &node.name, ports, root_vol, None, None),
        ..Default::default()
    }
}

struct Ports {
    pub main: String,
    pub grpc: String,
    pub mqtt: String,
    pub http: String,
}
fn vls_ports(idx: u16) -> Ports {
    let main_port = 9735 + idx;
    let grpc_port = 10019 + idx;
    let mqtt_port = 1883 + idx;
    let http_port = 5000 + idx;
    Ports {
        main: main_port.to_string(),
        grpc: grpc_port.to_string(),
        mqtt: mqtt_port.to_string(),
        http: http_port.to_string(),
    }
}

pub fn cln_vls(
    project: &str,
    name: &str,
    network: &str,
    idx: u16,
    btc: &BtcImage,
) -> Config<String> {
    let version = "0.1.5"; // docker tag
    let cln_version = "v0.11.0.1-793-g243f8e3";
    let ps = vls_ports(idx);
    let ports = vec![
        ps.main.clone(),
        ps.grpc.clone(),
        ps.mqtt.clone(),
        ps.http.clone(),
    ];
    let root_vol = "/root/.lightning";
    let links = Some(vec![domain(&btc.name)]);
    Config {
        image: Some(format!("sphinxlightning/sphinx-cln-vls:{}", version)),
        hostname: Some(domain(name)),
        domainname: Some(name.to_string()),
        cmd: Some(vec![
            format!("--alias=sphinx-{}", name),
            format!("--addr=0.0.0.0:{}", ps.main),
            format!("--grpc-port={}", ps.grpc),
            "--network=regtest".to_string(),
            "--bitcoin-rpcconnect=bitcoind".to_string(),
            "--bitcoin-rpcport=18443".to_string(),
            "--bitcoin-rpcuser=foo".to_string(),
            "--bitcoin-rpcpassword=bar".to_string(),
            "--log-level=debug".to_string(),
            "--accept-htlc-tlv-types=133773310".to_string(),
            "--subdaemon=hsmd:/usr/local/libexec/c-lightning/sphinx-key-broker".to_string(),
        ]),
        exposed_ports: exposed_ports(ports.clone()),
        env: Some(vec![
            "EXPOSE_TCP=true".to_string(),
            format!("GREENLIGHT_VERSION={cln_version}"),
            format!("LIGHTNINGD_PORT={}", ps.main),
            format!("LIGHTNINGD_NETWORK={}", network),
            format!("BROKER_MQTT_PORT={}", ps.mqtt),
            format!("BROKER_HTTP_PORT={}", ps.http),
        ]),
        host_config: host_config(project, name, ports, root_vol, None, links),
        ..Default::default()
    }
}

impl Image {
    pub fn as_btc(&self) -> anyhow::Result<BtcImage> {
        match self {
            Image::Btc(i) => Ok(i.clone()),
            _ => Err(anyhow::anyhow!("Not BTC".to_string())),
        }
    }
}
