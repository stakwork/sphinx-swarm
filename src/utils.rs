use crate::dock;
use anyhow::{anyhow, Result};
use bollard::container::NetworkingConfig;
use bollard::network::CreateNetworkOptions;
use bollard_stubs::models::{
    HostConfig, HostConfigLogConfig, Ipam, IpamConfig, PortBinding, PortMap, ResourcesUlimits,
    RestartPolicy, RestartPolicyNameEnum,
};
use rocket::tokio;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use tokio::{fs, io::AsyncWriteExt};

pub fn host_config(
    name: &str,
    ports: Vec<String>,
    root_vol: &str,
    extra_vols: Option<Vec<String>>,
) -> Option<HostConfig> {
    let mut dvols = vec![volume_string(name, root_vol)];
    if let Some(evs) = extra_vols {
        dvols.extend(evs);
    }
    Some(HostConfig {
        binds: Some(dvols),
        port_bindings: host_port(ports),
        extra_hosts: extra_hosts(),
        network_mode: Some(dock::DEFAULT_NETWORK.to_string()),
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::ON_FAILURE),
            maximum_retry_count: Some(100),
        }),
        ..Default::default()
    })
}

pub fn manual_host_config(
    ports: Vec<String>,
    vols: Option<Vec<String>>,
    add_ulimits: bool,
    add_log_limit: bool,
) -> Option<HostConfig> {
    let mut hc = HostConfig {
        binds: vols,
        port_bindings: host_port(ports),
        extra_hosts: extra_hosts(),
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
        typ: Some("json-file".to_string()),
        config: Some(config),
    })
}

fn extra_hosts() -> Option<Vec<String>> {
    Some(vec!["host.docker.internal:host-gateway".to_string()])
}

pub fn user() -> Option<String> {
    let uid = std::env::var("DOCKER_USER_ID");
    if let Ok(id) = uid {
        // Some(format!("{}:{}", id, id))
        Some(id)
    } else {
        None
    }
}

pub fn domain(name: &str) -> String {
    format!("{}.sphinx", name)
}

pub fn docker_domain(name: &str) -> String {
    if let Ok(_) = std::env::var("DOCKER_RUN") {
        domain(name)
    } else {
        "localhost".to_string()
    }
}

pub fn docker_domain_127(name: &str) -> String {
    if let Ok(_) = std::env::var("DOCKER_RUN") {
        domain(name)
    } else {
        "127.0.0.1".to_string()
    }
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

pub fn _volume_permissions(project: &str, name: &str, dir: &str) -> Result<()> {
    let perms = std::fs::Permissions::from_mode(0o777);
    let directory = format!("{}/{}", _host_volume_string(project, name), dir);
    std::fs::set_permissions(directory, perms).map_err(|e| anyhow!(e.to_string()))
}

pub fn _host_volume_string(project: &str, name: &str) -> String {
    let pwd = std::env::current_dir().unwrap_or_default();
    format!("{}/vol/{}/{}", pwd.to_string_lossy(), project, name)
}

// {vol_name}:{dir} ... vol_name = container domain
pub fn volume_string(name: &str, dir: &str) -> String {
    // ":z" is a fix for SELinux permissions. Can be shared
    format!(
        "{}:{}:rw", // "{}:{}:rw",
        domain(name),
        dir
    )
}

pub fn files_volume() -> String {
    let pwd = std::env::current_dir().unwrap_or_default();
    format!("{}/files:/files:z", pwd.to_string_lossy())
}

pub fn host_port(ports_in: Vec<String>) -> Option<PortMap> {
    let mut ports = PortMap::new();
    for port in ports_in {
        ports.insert(
            tcp_port(&port),
            Some(vec![PortBinding {
                host_port: Some(port.to_string()),
                // host_ip: Some("0.0.0.0".to_string()),
                host_ip: None,
            }]),
        );
    }
    Some(ports)
}

// from port 80 inside the container (like nginix)
pub fn single_host_port_from_eighty(port: &str) -> Option<PortMap> {
    let mut ports = PortMap::new();
    ports.insert(
        tcp_port(&"80"),
        Some(vec![PortBinding {
            host_port: Some(port.to_string()),
            // host_ip: Some("0.0.0.0".to_string()),
            host_ip: None,
        }]),
    );
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

pub async fn load_yaml<T: DeserializeOwned + Serialize>(file: &str, default: T) -> Result<T> {
    let path = std::path::Path::new(&file);
    match fs::read(path.clone()).await {
        Ok(data) => match serde_yaml::from_slice::<T>(&data) {
            Ok(d) => Ok(d),
            Err(e) => {
                log::warn!("error loading YAML {:?}", e);
                return Err(anyhow!("failed to load YAML config"));
            }
        },
        Err(_e) => {
            log::info!("creating a brand new default YAML config file!");
            let prefix = path.parent().unwrap();
            fs::create_dir_all(prefix).await.unwrap();
            put_yaml(file, &default).await;
            Ok(default)
        }
    }
}
pub async fn get_yaml<T: DeserializeOwned>(file: &str) -> T {
    let path = std::path::Path::new(&file);
    let data = fs::read(path.clone()).await.unwrap();
    serde_yaml::from_slice(&data).unwrap()
}
pub async fn put_yaml<T: Serialize>(file: &str, rs: &T) {
    let path = std::path::Path::new(&file);
    let st = serde_yaml::to_string(rs).expect("failed to make yaml string");
    let mut file = fs::File::create(path).await.expect("create failed");
    file.write_all(st.as_bytes()).await.expect("write failed");
}

pub async fn wait_for_file(path: &str, iterations: usize) -> Result<()> {
    for _ in 0..iterations {
        if std::path::Path::new(path).exists() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    Err(anyhow!(format!("{} does not exists", path)))
}

pub fn setup_logs() {
    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_module_level("bollard", log::LevelFilter::Warn)
        .with_module_level("want", log::LevelFilter::Off)
        .with_module_level("mio", log::LevelFilter::Off)
        .with_module_level("rocket", log::LevelFilter::Error)
        .with_module_level("hyper", log::LevelFilter::Warn)
        .with_module_level("tracing", log::LevelFilter::Error)
        .with_module_level("tokio_util", log::LevelFilter::Error)
        .with_module_level("tonic", log::LevelFilter::Error)
        .with_module_level("h2", log::LevelFilter::Error)
        .with_module_level("bitcoincore_rpc", log::LevelFilter::Error)
        .with_module_level("rustls", log::LevelFilter::Error)
        .with_module_level("tower", log::LevelFilter::Error)
        .with_module_level("reqwest", log::LevelFilter::Error)
        .with_module_level("_", log::LevelFilter::Error)
        .init()
        .unwrap();
}

pub async fn sleep_ms(n: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(n)).await;
}
