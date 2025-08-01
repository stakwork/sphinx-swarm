use crate::dock;
use anyhow::{anyhow, Result};
use bollard::container::NetworkingConfig;
use bollard::models::DeviceRequest;
use bollard::network::CreateNetworkOptions;
use bollard_stubs::models::{
    HostConfig, HostConfigLogConfig, Ipam, IpamConfig, PortBinding, PortMap, ResourcesUlimits,
    RestartPolicy, RestartPolicyNameEnum,
};
use rocket::tokio;
use serde::{de::DeserializeOwned, Serialize};
use std::fs::{read_to_string, write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::{collections::HashMap, time::Duration};
use tokio::{fs, io::AsyncWriteExt};

pub fn host_config(
    name: &str,
    ports: Vec<String>,
    root_vol: &str,
    extra_vols: Option<Vec<String>>,
    mem_limit: Option<i64>,
) -> Option<HostConfig> {
    let mut dvols = vec![volume_string(name, root_vol)];
    if let Some(evs) = extra_vols {
        dvols.extend(evs);
    }
    let mut hc = HostConfig {
        binds: Some(dvols),
        port_bindings: host_port(ports),
        extra_hosts: extra_hosts(),
        network_mode: Some(dock::DEFAULT_NETWORK.to_string()),
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
            maximum_retry_count: None,
        }),
        log_config: local_log_config(),
        ..Default::default()
    };
    if let Some(ml) = mem_limit {
        hc.memory = Some(ml);
    } else {
        use std::sync::atomic::Ordering;
        let global_mem_limit = crate::config::GLOBAL_MEM_LIMIT.load(Ordering::Relaxed);
        if global_mem_limit > 0 {
            hc.memory = Some(global_mem_limit as i64);
        }
    }
    Some(hc)
}

pub fn add_gpus_to_host_config(hc: &mut HostConfig, count: i64) {
    hc.device_requests = Some(vec![DeviceRequest {
        driver: Some("nvidia".to_string()),
        count: Some(count),
        capabilities: Some(vec![vec!["gpu".to_string()]]),
        ..Default::default()
    }]);
}

fn local_log_config() -> Option<HostConfigLogConfig> {
    let mut h = HashMap::new();
    h.insert("max-size".to_string(), "10m".to_string());
    h.insert("max-file".to_string(), "5".to_string());
    Some(HostConfigLogConfig {
        typ: Some("local".to_string()),
        config: Some(h),
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

pub fn docker_domain_tonic(name: &str) -> String {
    if let Ok(_) = std::env::var("DOCKER_RUN") {
        domain(name)
    } else {
        "[::1]".to_string()
    }
}

pub fn is_using_port_based_ssl() -> bool {
    if let Ok(pbs) = std::env::var("PORT_BASED_SSL") {
        if pbs == "true" || pbs == "1" {
            return true;
        }
    }
    false
}

fn filter_out_reserved_ports_if_using_port_based_ssl(ports: Vec<String>) -> Vec<String> {
    if !is_using_port_based_ssl() {
        return ports;
    }
    ports
        .into_iter()
        .filter(|p| p != "7799" && p != "3355" && p != "8000" && p != "6000" && p != "8444")
        .collect()
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

pub fn host_port(mut ports_in: Vec<String>) -> Option<PortMap> {
    ports_in = filter_out_reserved_ports_if_using_port_based_ssl(ports_in);
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
pub fn single_host_port_from(port: &str, from_port: &str) -> Option<PortMap> {
    let mut ports = PortMap::new();
    ports.insert(
        tcp_port(from_port),
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
    match fs::read(path).await {
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
    let data = fs::read(path).await.unwrap();
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
    match fs::read(path).await {
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
    let data = fs::read(path).await.unwrap();
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

pub fn make_reqwest_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build reqwest client")
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
        .with_module_level("aws-sdk-s3", log::LevelFilter::Error)
        .with_module_level("aws-config", log::LevelFilter::Error)
        .with_module_level("aws-smithy-types", log::LevelFilter::Error)
        .with_module_level("aws_smithy_runtime", log::LevelFilter::Error)
        .with_module_level("aws_runtime", log::LevelFilter::Error)
        .with_module_level("aws_sdk_sts", log::LevelFilter::Error)
        .with_module_level("_", log::LevelFilter::Error)
        .init()
        .unwrap();
}

pub async fn sleep_ms(n: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(n)).await;
}

pub fn getenv(envname: &str) -> Result<String> {
    let sh = std::env::var(envname)?;
    // remove empty string
    if sh.len() > 0 {
        Ok(sh)
    } else {
        Err(anyhow!("{} is empty", envname))
    }
}

pub fn extract_swarm_number(host: String) -> String {
    host.chars().filter(|c| c.is_numeric()).collect()
}

pub fn update_or_write_to_env_file(updates: &HashMap<String, String>) -> Result<()> {
    let env_path_string = ".env";
    let env_path = Path::new(env_path_string);

    // Read existing file
    let content = if env_path.exists() {
        read_to_string(env_path)?
    } else {
        log::error!("Could not find env file at path: {}", env_path_string);
        return Err(anyhow!(format!(
            "Could not find env file at path: {}",
            env_path_string
        )));
    };

    // Parse into map
    let mut lines: Vec<String> = Vec::new();
    let mut keys_handled = HashMap::new();

    for line in content.lines() {
        if let Some((key, _)) = line.split_once('=') {
            if let Some(new_val) = updates.get(key.trim()) {
                lines.push(format!("{}={}", key.trim(), new_val));
                keys_handled.insert(key.trim().to_string(), true);
            } else {
                lines.push(line.to_string());
            }
        } else {
            lines.push(line.to_string()); // comments or empty lines
        }
    }

    // Add any new keys that weren't in the original file
    for (k, v) in updates {
        if !keys_handled.contains_key(k) {
            lines.push(format!("{}={}", k, v));
        }
    }

    // Write back
    write(env_path, lines.join("\n"))?;

    Ok(())
}
