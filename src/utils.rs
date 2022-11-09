use bollard::container::NetworkingConfig;
use bollard::network::CreateNetworkOptions;
use bollard_stubs::models::{HostConfig, Ipam, IpamConfig, PortBinding, PortMap};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;

pub fn host_config(
    project: &str,
    name: &str,
    ports: Vec<&str>,
    root_vol: &str,
    extra_vols: Option<Vec<String>>,
    links: Option<Vec<String>>,
) -> Option<HostConfig> {
    let mut dvols = vec![volume_string(project, name, root_vol)];
    if let Some(evs) = extra_vols {
        dvols.extend(evs);
    }
    Some(HostConfig {
        binds: Some(dvols),
        port_bindings: host_port(ports),
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
        links,
        ..Default::default()
    })
}

pub fn domain(name: &str) -> String {
    format!("{}.sphinx", name)
}

pub fn exposed_ports(ports: Vec<&str>) -> Option<HashMap<String, HashMap<(), ()>>> {
    let mut ps = HashMap::new();
    for port in ports {
        ps.insert(tcp_port(port), HashMap::new());
    }
    Some(ps)
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

// DIR/vol/{project}/{container_name}:{dir}
pub fn volume_string(project: &str, name: &str, dir: &str) -> String {
    let pwd = std::env::current_dir().unwrap_or_default();
    format!("{}/vol/{}/{}:{}", pwd.to_string_lossy(), project, name, dir)
}

pub fn files_volume() -> String {
    let pwd = std::env::current_dir().unwrap_or_default();
    format!("{}/files:/files", pwd.to_string_lossy())
}

fn host_port(ports_in: Vec<&str>) -> Option<PortMap> {
    let mut ports = PortMap::new();
    for port in ports_in {
        ports.insert(
            tcp_port(port),
            Some(vec![PortBinding {
                host_port: Some(port.to_string()),
                host_ip: Some("0.0.0.0".to_string()),
                // host_ip: None,
            }]),
        );
    }
    Some(ports)
}

pub fn _custom_network() -> CreateNetworkOptions<String> {
    CreateNetworkOptions {
        name: _NET.to_string(),
        driver: "default".to_string(),
        attachable: true,
        ipam: Ipam {
            driver: Some("default".to_string()),
            config: Some(vec![IpamConfig {
                subnet: Some("172.18.0.0/16".to_string()),
                gateway: Some("172.18.0.1".to_string()),
                ip_range: Some("172.18.5.0/24".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub const _NET: &str = "network1";

pub fn _net_config() -> Option<NetworkingConfig<String>> {
    let mut endpoints_config = HashMap::new();
    endpoints_config.insert(_NET.to_string(), Default::default());
    Some(NetworkingConfig { endpoints_config })
}

pub fn load_json<T: DeserializeOwned + Serialize>(file: &str, default: T) -> T {
    let path = std::path::Path::new(&file);
    match std::fs::read(path.clone()) {
        Ok(data) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => default,
        },
        Err(_e) => {
            let prefix = path.parent().unwrap();
            std::fs::create_dir_all(prefix).unwrap();
            put_json(file, &default);
            default
        }
    }
}
pub fn get_json<T: DeserializeOwned>(file: &str) -> T {
    let path = std::path::Path::new(&file);
    let data = std::fs::read(path.clone()).unwrap();
    serde_json::from_slice(&data).unwrap()
}
pub fn put_json<T: Serialize>(file: &str, rs: &T) {
    use std::io::Write;
    let path = std::path::Path::new(&file);
    let st = serde_json::to_string_pretty(rs).expect("failed to make json string");
    let mut file = std::fs::File::create(path).expect("create failed");
    file.write_all(st.as_bytes()).expect("write failed");
}
