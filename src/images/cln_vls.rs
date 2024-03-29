use super::*;
use crate::utils::{domain, exposed_ports, host_config};
use bollard::container::Config;

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

fn cln_vls(name: &str, network: &str, idx: u16, btc: &btc::BtcImage) -> Config<String> {
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
    let mut cmd = vec![
        format!("--alias=sphinx-{}", name),
        format!("--addr=0.0.0.0:{}", ps.main),
        format!("--grpc-port={}", ps.grpc),
        "--network=regtest".to_string(),
        format!("--bitcoin-rpcconnect={}", &domain(&btc.name)),
        "--bitcoin-rpcport=18443".to_string(),
        "--log-level=debug".to_string(),
        "--accept-htlc-tlv-types=133773310".to_string(),
        "--subdaemon=hsmd:/usr/local/libexec/c-lightning/sphinx-key-broker".to_string(),
    ];
    if let Some(u) = &btc.user {
        if let Some(p) = &btc.pass {
            cmd.push(format!("--bitcoin-rpcuser={}", u));
            cmd.push(format!("--bitcoin-rpcpassword={}", p));
        }
    }
    Config {
        image: Some(format!("sphinxlightning/sphinx-cln-vls:{}", version)),
        hostname: Some(domain(name)),
        domainname: Some(name.to_string()),
        cmd: Some(cmd),
        exposed_ports: exposed_ports(ports.clone()),
        env: Some(vec![
            "EXPOSE_TCP=true".to_string(),
            format!("GREENLIGHT_VERSION={cln_version}"),
            format!("LIGHTNINGD_PORT={}", ps.main),
            format!("LIGHTNINGD_NETWORK={}", network),
            format!("BROKER_MQTT_PORT={}", ps.mqtt),
            format!("BROKER_HTTP_PORT={}", ps.http),
        ]),
        host_config: host_config(name, ports, root_vol, None),
        ..Default::default()
    }
}
