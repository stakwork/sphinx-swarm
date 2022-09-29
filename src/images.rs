use crate::utils::{expose, host_config};
use bollard::container::Config;

// ports are tcp
// volumes are mapped to {PWD}/vol/{name}:

pub fn btc<'a>(name: &'a str) -> Config<&'a str> {
    let ports = vec!["18443"];
    let vols = vec!["/home/bitcoin/.bitcoin"];
    Config {
        image: Some("ruimarinho/bitcoin-core:23.0"),
        hostname: Some(name),
        cmd: Some(vec![
            "-regtest=1",
            "-rpcallowip=0.0.0.0/0",
            "-rpcbind=0.0.0.0",
            "-rpcpassword=bar",
            "-rpcport=18443",
            "-rpcuser=foo",
            "-server",
        ]),
        host_config: host_config(name, ports, vols, None),
        ..Default::default()
    }
}

struct Ports {
    pub main: String,
    pub grpc: String,
    pub mqtt: String,
    pub rocket: String,
}
fn vls_ports(idx: u16) -> Ports {
    let main_port = 9735 + idx;
    let grpc_port = 10019 + idx;
    let mqtt_port = 1883 + idx;
    let rocket_port = 5000 + idx;
    Ports {
        main: main_port.to_string(),
        grpc: grpc_port.to_string(),
        mqtt: mqtt_port.to_string(),
        rocket: rocket_port.to_string(),
    }
}

pub fn cln_vls(name: &str, idx: u16, links: Vec<&str>) -> Config<String> {
    let ps = vls_ports(idx);
    let ports = vec![
        ps.main.as_str(),
        ps.grpc.as_str(),
        ps.mqtt.as_str(),
        ps.rocket.as_str(),
    ];
    let vols = vec!["/root/.lightning"];
    Config {
        image: Some("sphinxlightning/sphinx-cln-vls:0.1.3".to_string()),
        hostname: Some(name.to_string()),
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
            "GREENLIGHT_VERSION=v0.11.0.1-792-g17cc61c".to_string(),
            format!("LIGHTNINGD_PORT={}", ps.main).to_string(),
            "LIGHTNINGD_NETWORK=regtest".to_string(),
            format!("BROKER_PORT={}", ps.mqtt).to_string(),
            format!("ROCKET_PORT={}", ps.rocket).to_string(),
        ]),
        host_config: host_config(name, ports, vols, Some(links)),
        ..Default::default()
    }
}
