use crate::utils::{expose, host_config};
use bollard::container::Config;

// ports are tcp
// volumes are mapped to {PWD}/vol/{name}:

pub fn btc(name: &str, network: &str) -> Config<String> {
    let btc_version = "23.0";
    let ports = vec!["18443"];
    let vols = vec!["/home/bitcoin/.bitcoin"];
    Config {
        image: Some(format!("ruimarinho/bitcoin-core:{}", btc_version)),
        hostname: Some(name.to_string()),
        cmd: Some(vec![
            format!("-{}=1", network),
            "-rpcallowip=0.0.0.0/0".to_string(),
            "-rpcbind=0.0.0.0".to_string(),
            "-rpcpassword=bar".to_string(),
            "-rpcport=18443".to_string(),
            "-rpcuser=foo".to_string(),
            "-server".to_string(),
        ]),
        host_config: host_config(name, ports, vols, None),
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

pub fn cln_vls(name: &str, idx: u16, links: Vec<&str>, network: &str) -> Config<String> {
    let version = "0.1.5"; // docker tag
    let cln_version = "v0.11.0.1-793-g243f8e3";
    let ps = vls_ports(idx);
    let ports = vec![
        ps.main.as_str(),
        ps.grpc.as_str(),
        ps.mqtt.as_str(),
        ps.http.as_str(),
    ];
    let vols = vec!["/root/.lightning"];
    Config {
        image: Some(format!("sphinxlightning/sphinx-cln-vls:{}", version)),
        hostname: Some(name.to_string()),
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
        exposed_ports: expose(ports.clone()),
        env: Some(vec![
            "EXPOSE_TCP=true".to_string(),
            format!("GREENLIGHT_VERSION={cln_version}"),
            format!("LIGHTNINGD_PORT={}", ps.main),
            format!("LIGHTNINGD_NETWORK={}", network),
            format!("BROKER_MQTT_PORT={}", ps.mqtt),
            format!("BROKER_HTTP_PORT={}", ps.http),
        ]),
        host_config: host_config(name, ports, vols, Some(links)),
        ..Default::default()
    }
}
