use bollard::container::NetworkingConfig;
use bollard::network::CreateNetworkOptions;
use bollard_stubs::models::{EndpointSettings, HostConfig, Ipam, IpamConfig, PortBinding, PortMap};
use std::collections::HashMap;

pub fn host_config(
    name: &str,
    ports: Vec<&str>,
    vols: Vec<&str>,
    links: Option<Vec<&str>>,
) -> Option<HostConfig> {
    let mut c = HostConfig {
        binds: volumes(name, vols),
        port_bindings: host_port(ports),
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
        ..Default::default()
    };
    if let Some(ls) = links {
        c.links = Some(ls.iter().map(|l| l.to_string()).collect());
    }
    Some(c)
}

fn tcp_port(p: &str) -> String {
    format!("{}/tcp", p).to_string()
}

pub fn expose(ports: Vec<&str>) -> Option<HashMap<String, HashMap<(), ()>>> {
    let mut h = HashMap::new();
    for p in ports {
        h.insert(tcp_port(p), HashMap::<(), ()>::new());
    }
    Some(h)
}

fn volumes(name: &str, ins: Vec<&str>) -> Option<Vec<String>> {
    let pwd = std::env::current_dir().unwrap_or_default();
    let mut fulls: Vec<String> = Vec::new();
    for i in ins {
        let path = format!("{}/vol/{}:{}", pwd.to_string_lossy(), name, i);
        fulls.push(path);
    }
    Some(fulls)
}

fn host_port(ports_in: Vec<&str>) -> Option<PortMap> {
    let mut ports = PortMap::new();
    for port in ports_in {
        ports.insert(
            tcp_port(port),
            Some(vec![PortBinding {
                host_port: Some(port.to_string()),
                // host_ip: None,
                host_ip: Some("0.0.0.0".to_string()),
            }]),
        );
    }
    Some(ports)
}

pub fn _bridge_network() -> CreateNetworkOptions<String> {
    CreateNetworkOptions {
        name: _NET.to_string(),
        driver: "bridge".to_string(),
        ipam: Ipam {
            driver: Some("default".to_string()),
            config: Some(vec![IpamConfig {
                subnet: Some("172.17.0.0/16".to_string()),
                gateway: Some("172.17.0.1".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub const _NET: &str = "bridge";

pub fn _net_config(alias: &str, idx: u8) -> Option<NetworkingConfig<String>> {
    println!("{:?}", idx.to_be_bytes());
    let mac_end = hex::encode(idx.to_be_bytes());
    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(
        _NET.to_string(),
        EndpointSettings {
            ip_address: Some(format!("172.17.0.{}/16", idx)),
            mac_address: Some(format!("02:42:ac:11:00:{}", mac_end)),
            aliases: Some(vec![alias.to_string()]),
            ..Default::default() // links=['container2']
        },
    );
    Some(NetworkingConfig { endpoints_config })
}
