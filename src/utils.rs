use bollard::container::NetworkingConfig;
use bollard::network::CreateNetworkOptions;
use bollard_stubs::models::{
    HostConfig, HostConfigLogConfig, Ipam, IpamConfig, PortBinding, PortMap, ResourcesUlimits,
};
use rocket::tokio::fs;
use rocket::tokio::io::AsyncWriteExt;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::env;

pub fn host_config(
    project: &str,
    name: &str,
    ports: Vec<String>,
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
        extra_hosts: extra_hosts(),
        links,
        ..Default::default()
    })
}

pub fn manual_host_config(
    ports: Vec<String>,
    vols: Option<Vec<String>>,
    links: Option<Vec<String>>,
    add_ulimits: bool,
    add_log_limit: bool,
) -> Option<HostConfig> {
    let mut hc = HostConfig {
        binds: vols,
        port_bindings: host_port(ports),
        extra_hosts: extra_hosts(),
        links,
        ..Default::default()
    };
    if add_ulimits {
        hc.ulimits = ulimits();
    }
    if add_log_limit {
        hc.log_config = log_limit();
    }
    Some(hc)
}

fn ulimits() -> Option<Vec<ResourcesUlimits>> {
    Some(vec![
        ResourcesUlimits {
            name: Some("nproc".to_string()),
            soft: Some(65535),
            hard: Some(65535),
        },
        ResourcesUlimits {
            name: Some("nofile".to_string()),
            soft: Some(1000000),
            hard: Some(1000000),
        },
    ])
}

fn log_limit() -> Option<HostConfigLogConfig> {
    let mut config = HashMap::new();
    config.insert("max-size".to_string(), "10m".to_string());
    Some(HostConfigLogConfig {
        typ: Some("options".to_string()),
        config: Some(config),
    })
}

fn extra_hosts() -> Option<Vec<String>> {
    Some(vec!["host.docker.internal:host-gateway".to_string()])
}

pub fn user() -> Option<String> {
    let uid = std::env::var("DOCKER_USER_ID");
    if let Ok(id) = uid {
        Some(format!("{}:{}", id, id))
    } else {
        None
    }
}

pub fn domain(name: &str) -> String {
    format!("{}.sphinx", name)
}

pub fn exposed_ports(ports: Vec<String>) -> Option<HashMap<String, HashMap<(), ()>>> {
    let mut ps = HashMap::new();
    for port in ports {
        ps.insert(tcp_port(&port), HashMap::new());
    }
    Some(ps)
}

fn tcp_port(p: &str) -> String {
    format!("{}/tcp", p).to_string()
}

// DIR/vol/{project}/{container_name}:{dir}
pub fn volume_string(project: &str, name: &str, dir: &str) -> String {
    let pwd = std::env::current_dir().unwrap_or_default();
    // ":z" is a fix for SELinux permissions. Can be shared
    format!(
        "{}/vol/{}/{}:{}:z",
        pwd.to_string_lossy(),
        project,
        name,
        dir
    )
}

pub fn files_volume() -> String {
    let pwd = std::env::current_dir().unwrap_or_default();
    format!("{}/files:/files:z", pwd.to_string_lossy())
}

fn host_port(ports_in: Vec<String>) -> Option<PortMap> {
    let mut ports = PortMap::new();
    for port in ports_in {
        ports.insert(
            tcp_port(&port),
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

pub async fn load_json<T: DeserializeOwned + Serialize>(file: &str, default: T) -> T {
    let path = std::path::Path::new(&file);
    match fs::read(path.clone()).await {
        Ok(data) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => default,
        },
        Err(_e) => {
            let prefix = path.parent().unwrap();
            fs::create_dir_all(prefix).await.unwrap();
            put_json(file, &default).await;
            default
        }
    }
}
pub async fn get_json<T: DeserializeOwned>(file: &str) -> T {
    let path = std::path::Path::new(&file);
    let data = fs::read(path.clone()).await.unwrap();
    serde_json::from_slice(&data).unwrap()
}
pub async fn put_json<T: Serialize>(file: &str, rs: &T) {
    let path = std::path::Path::new(&file);
    let st = serde_json::to_string_pretty(rs).expect("failed to make json string");
    let mut file = fs::File::create(path).await.expect("create failed");
    file.write_all(st.as_bytes()).await.expect("write failed");
}
