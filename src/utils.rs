use crate::dock;
use anyhow::{anyhow, Result};
use bollard::container::NetworkingConfig;
use bollard::models::{
    DeviceRequest, HostConfig, HostConfigLogConfig, Ipam, IpamConfig, PortBinding, PortMap,
    ResourcesUlimits, RestartPolicy, RestartPolicyNameEnum,
};
use bollard::network::CreateNetworkOptions;
use rocket::tokio;
use serde::{de::DeserializeOwned, Serialize};
use std::env;
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
    let swarm_number = getenv("SWARM_NUMBER").unwrap_or("".to_string());

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
        log_config: local_log_config(&swarm_number),
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

fn local_log_config(swarm_id: &str) -> Option<HostConfigLogConfig> {
    let mut h = HashMap::new();

    match get_environment() {
        Environment::Local => {
            // Local logging - logs to local files
            h.insert("max-size".to_string(), "10m".to_string());
            h.insert("max-file".to_string(), "3".to_string());

            Some(HostConfigLogConfig {
                typ: Some("json-file".to_string()),
                config: Some(h),
            })
        }
        Environment::Cloud => {
            // AWS CloudWatch logging
            h.insert("awslogs-region".to_string(), "us-east-1".to_string());
            h.insert("awslogs-group".to_string(), format!("/swarms/{}", swarm_id));
            h.insert("awslogs-create-group".to_string(), "true".to_string());
            h.insert("tag".to_string(), "{{.Name}}".to_string());
            h.insert("cache-max-size".to_string(), "10m".to_string());
            h.insert("cache-max-file".to_string(), "3".to_string());

            Some(HostConfigLogConfig {
                typ: Some("awslogs".to_string()),
                config: Some(h),
            })
        }
        Environment::Fluentd => {
            let address = env::var("FLUENTD_ADDRESS")
                .unwrap_or_else(|_| "localhost:24224".to_string());
            let tag = env::var("FLUENTD_TAG")
                .unwrap_or_else(|_| "{{.Name}}".to_string());
            // Docker fluentd-buffer-limit is bytes (Atoi); default matches
            // the daemon's 1MiB so a sidecar outage cannot grow unbounded.
            let buffer_limit = env::var("FLUENTD_BUFFER_LIMIT")
                .unwrap_or_else(|_| "1048576".to_string());

            h.insert("fluentd-address".to_string(), address);
            h.insert("tag".to_string(), tag);
            h.insert("fluentd-async".to_string(), "true".to_string());
            h.insert("fluentd-buffer-limit".to_string(), buffer_limit);
            // Same dual-logging cache as Cloud so docker.logs() (in-app
            // viewers) still returns output for the remote fluentd driver.
            h.insert("cache-max-size".to_string(), "10m".to_string());
            h.insert("cache-max-file".to_string(), "3".to_string());

            Some(HostConfigLogConfig {
                typ: Some("fluentd".to_string()),
                config: Some(h),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize all env-var-dependent tests: env vars are process-global and
    // cannot be safely read/written from parallel threads.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_fluentd_with_explicit_env_vars() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("RUST_ENV", "fluentd");
        env::set_var("FLUENTD_ADDRESS", "vector.example.com:24224");
        env::set_var("FLUENTD_TAG", "myapp");
        env::set_var("FLUENTD_BUFFER_LIMIT", "8388608");

        let config = local_log_config("test-swarm").unwrap();
        assert_eq!(config.typ, Some("fluentd".to_string()));
        let cfg = config.config.unwrap();
        assert_eq!(cfg.get("fluentd-address").unwrap(), "vector.example.com:24224");
        assert_eq!(cfg.get("tag").unwrap(), "myapp");
        assert_eq!(cfg.get("fluentd-async").unwrap(), "true");
        assert_eq!(cfg.get("fluentd-buffer-limit").unwrap(), "8388608");
        assert_eq!(cfg.get("cache-max-size").unwrap(), "10m");
        assert_eq!(cfg.get("cache-max-file").unwrap(), "3");

        env::remove_var("RUST_ENV");
        env::remove_var("FLUENTD_ADDRESS");
        env::remove_var("FLUENTD_TAG");
        env::remove_var("FLUENTD_BUFFER_LIMIT");
    }

    #[test]
    fn test_fluentd_with_defaults() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("RUST_ENV", "fluentd");
        env::remove_var("FLUENTD_ADDRESS");
        env::remove_var("FLUENTD_TAG");
        env::remove_var("FLUENTD_BUFFER_LIMIT");

        let config = local_log_config("test-swarm").unwrap();
        assert_eq!(config.typ, Some("fluentd".to_string()));
        let cfg = config.config.unwrap();
        assert_eq!(cfg.get("fluentd-address").unwrap(), "localhost:24224");
        assert_eq!(cfg.get("tag").unwrap(), "{{.Name}}");
        assert_eq!(cfg.get("fluentd-async").unwrap(), "true");
        assert_eq!(cfg.get("fluentd-buffer-limit").unwrap(), "1048576");
        assert_eq!(cfg.get("cache-max-size").unwrap(), "10m");
        assert_eq!(cfg.get("cache-max-file").unwrap(), "3");

        env::remove_var("RUST_ENV");
    }

    #[test]
    fn test_local_returns_json_file() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("RUST_ENV", "local");

        let config = local_log_config("test-swarm").unwrap();
        assert_eq!(config.typ, Some("json-file".to_string()));

        env::remove_var("RUST_ENV");
    }

    #[test]
    fn test_production_returns_awslogs() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("RUST_ENV", "production");

        let config = local_log_config("test-swarm").unwrap();
        assert_eq!(config.typ, Some("awslogs".to_string()));

        env::remove_var("RUST_ENV");
    }
}

#[derive(Debug, PartialEq)]
enum Environment {
    Local,
    Cloud,
    Fluentd,
}

fn get_environment() -> Environment {
    // Priority 1: Explicit environment variable
    if let Ok(env) = env::var("RUST_ENV") {
        match env.to_lowercase().as_str() {
            "local" | "development" | "dev" => return Environment::Local,
            "production" | "prod" | "cloud" => return Environment::Cloud,
            "fluentd" => return Environment::Fluentd,
            _ => {}
        }
    }

    Environment::Cloud
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
    if name == "swarm" {
        return format!("sphinx-swarm");
    }
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
        .filter(|p| {
            p != "7799"
                && p != "3355"
                && p != "8000"
                && p != "3100"
                && p != "6000"
                && p != "8444"
                && p != "9000"
                && p != "3333"
                && p != "8181"
        })
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

/// Networks a container joins besides its `network_mode`. They ride on the
/// config's `networking_config`; `dock::create_container` takes them off and
/// connects each one once the container exists.
pub fn extra_networks(names: Vec<&str>) -> Option<NetworkingConfig<String>> {
    let mut endpoints_config = HashMap::new();
    for name in names {
        endpoints_config.insert(name.to_string(), Default::default());
    }
    Some(NetworkingConfig { endpoints_config })
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
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build reqwest client")
}

pub fn setup_logs() {
    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_level(log::LevelFilter::Info)
        .with_module_level("bollard", log::LevelFilter::Warn)
        .with_module_level("want", log::LevelFilter::Off)
        .with_module_level("mio", log::LevelFilter::Off)
        .with_module_level("rocket", log::LevelFilter::Error)
        .with_module_level("hyper", log::LevelFilter::Warn)
        .with_module_level("tracing", log::LevelFilter::Error)
        .with_module_level("tokio_util", log::LevelFilter::Error)
        .with_module_level("tonic", log::LevelFilter::Error)
        .with_module_level("h2", log::LevelFilter::Warn)
        .with_module_level("bitcoincore_rpc", log::LevelFilter::Error)
        .with_module_level("rustls", log::LevelFilter::Warn)
        .with_module_level("tower", log::LevelFilter::Error)
        .with_module_level("reqwest", log::LevelFilter::Warn)
        .with_module_level("aws_sdk_s3", log::LevelFilter::Warn)
        .with_module_level("aws_config", log::LevelFilter::Warn)
        .with_module_level("aws_smithy_types", log::LevelFilter::Info)
        .with_module_level("aws_sigv4", log::LevelFilter::Warn)
        .with_module_level("aws_smithy_runtime", log::LevelFilter::Warn)
        .with_module_level("aws_runtime", log::LevelFilter::Warn)
        .with_module_level("aws_sdk_sts", log::LevelFilter::Error)
        .with_module_level("aws_smithy_http", log::LevelFilter::Warn)
        .with_module_level("aws_smithy_runtime_api", log::LevelFilter::Warn)
        .with_module_level("aws_sdk_ec2", log::LevelFilter::Warn)
        .with_module_level("aws_sdk_route53", log::LevelFilter::Warn)
        .with_module_level("aws_sdk_cloudwatchlogs", log::LevelFilter::Warn)
        .with_module_level("aws_sdk_cloudwatch", log::LevelFilter::Warn)
        .with_module_level("aws_sdk_sns", log::LevelFilter::Warn)
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
    if let Some(pos) = host.find("swarm") {
        host[pos + 5..].to_string()
    } else {
        String::new()
    }
}

pub fn update_or_write_to_env_file(updates: &HashMap<String, String>) -> Result<()> {
    update_env_file(Path::new(".env"), updates)
}

fn update_env_file(env_path: &Path, updates: &HashMap<String, String>) -> Result<()> {
    // Read existing file
    let content = if env_path.exists() {
        read_to_string(env_path)?
    } else {
        log::error!("Could not find env file at path: {}", env_path.display());
        return Err(anyhow!(format!(
            "Could not find env file at path: {}",
            env_path.display()
        )));
    };

    // Parse into map
    let mut lines: Vec<String> = Vec::new();
    let mut keys_handled = HashMap::new();

    for line in content.lines() {
        if let Some((key, _)) = line.split_once('=') {
            if let Some(new_val) = updates.get(key.trim()) {
                lines.push(format!("{}={}", key.trim(), env_value(new_val)));
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
            lines.push(format!("{}={}", k, env_value(v)));
        }
    }

    // Write back, ending in a newline so a later `>> .env` starts its own line
    write(env_path, lines.join("\n") + "\n")?;

    Ok(())
}

/// `value` written so dotenv 0.15 reads it back exactly. Unquoted, a space
/// is a parse error and `dotenv()` drops that line and every line below it
/// (2026-11-17: `TWITTER_BEARER=Bearer xyz` cost 11 swarms OWNER_PUBKEY and
/// the rest on restart). Single quotes keep everything but `'` and a newline;
/// double quotes take `\\`, `\"`, `\$` and `\n` escapes.
pub fn env_value(value: &str) -> String {
    if !value.contains(|c: char| c.is_whitespace() || "'\"\\$#".contains(c)) {
        return value.to_string();
    }
    if !value.contains(['\'', '\n']) {
        return format!("'{}'", value);
    }
    let mut quoted = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' | '"' | '$' => {
                quoted.push('\\');
                quoted.push(c);
            }
            '\n' => quoted.push_str("\\n"),
            _ => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// Load the `.env` that `dotenv::dotenv()` finds (the working directory or
/// a parent), past any line it can't parse. `dotenv()` returns at the first
/// bad line, and `.ok()` hid it, so no key below that line was ever set.
pub fn load_dotenv() {
    let dir = env::current_dir().unwrap_or_default();
    if let Some(path) = dir
        .ancestors()
        .map(|d| d.join(".env"))
        .find(|p| p.is_file())
    {
        load_env_file(&path);
    }
}

/// Set each pair in `path` the way dotenv's `Iter::load` does (only if the
/// variable isn't set yet), and log each line it can't parse by number and
/// key, never the value.
#[allow(deprecated)]
pub fn load_env_file(path: &Path) {
    let Ok(pairs) = dotenv::from_path_iter(path) else {
        return;
    };
    // dotenv yields one item for each line that is not blank or a comment
    let content = String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default()).into_owned();
    let mut lines = content.lines().enumerate().filter(|(_, l)| {
        let l = l.trim();
        !l.is_empty() && !l.starts_with('#')
    });
    for pair in pairs {
        let (i, line) = lines.next().unwrap_or_default();
        match pair {
            Ok((key, value)) => {
                if env::var(&key).is_err() {
                    env::set_var(&key, value);
                }
            }
            Err(_) => {
                let key = line.split_once('=').map_or("", |(k, _)| k.trim());
                let key = key.strip_prefix("export ").unwrap_or(key).trim();
                let key_like = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
                let key = if !key.is_empty() && key.chars().all(key_like) {
                    key
                } else {
                    "?"
                };
                eprintln!(
                    "{}: line {} ({}) can't be parsed, skipped",
                    path.display(),
                    i + 1,
                    key
                );
            }
        }
    }
}

#[cfg(test)]
mod env_file_tests {
    use super::*;

    fn temp_env(name: &str, content: &str) -> std::path::PathBuf {
        let path = env::temp_dir().join(format!("swarm-{}-{}.env", name, std::process::id()));
        write(&path, content).unwrap();
        path
    }

    #[allow(deprecated)]
    fn read_back(path: &Path) -> HashMap<String, String> {
        dotenv::from_path_iter(path)
            .unwrap()
            .map(|p| p.unwrap())
            .collect()
    }

    #[test]
    fn values_read_back_exactly() {
        let values = [
            "plain",
            "",
            "a=b",
            "Bearer xyz",
            "  padded  ",
            "tab\there",
            "--max-old-space-size=4096 --expose-gc",
            "it's",
            "say \"hi\"",
            "back\\slash",
            "$HOME and ${PATH}",
            "#not a comment",
            "a#b",
            "two\nlines",
            "it's $5 \"really\" \\ ok\n",
        ];
        let updates: HashMap<String, String> = values
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("SWARM_TEST_{}", i), v.to_string()))
            .collect();
        let path = temp_env("values", "# swarm\nKEPT=1\nSWARM_TEST_0=old");
        update_env_file(&path, &updates).unwrap();

        let content = read_to_string(&path).unwrap();
        assert!(content.starts_with("# swarm\nKEPT=1\nSWARM_TEST_0=plain\n"));
        assert!(content.ends_with('\n'));
        let mut read = read_back(&path);
        assert_eq!(read.remove("KEPT").as_deref(), Some("1"));
        assert_eq!(read, updates);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_bad_line_does_not_hide_the_lines_below() {
        let path = temp_env(
            "bad-line",
            "SWARM_LOAD_A=1\n# comment\n\nSWARM_LOAD_BAD=Bearer xyz\nSWARM_LOAD_B='b c'\n",
        );
        load_env_file(&path);
        assert_eq!(env::var("SWARM_LOAD_A").as_deref(), Ok("1"));
        assert!(env::var("SWARM_LOAD_BAD").is_err());
        assert_eq!(env::var("SWARM_LOAD_B").as_deref(), Ok("b c"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn load_keeps_variables_already_set() {
        env::set_var("SWARM_LOAD_SET", "from the process");
        let path = temp_env("already-set", "SWARM_LOAD_SET=from the file\n");
        load_env_file(&path);
        assert_eq!(
            env::var("SWARM_LOAD_SET").as_deref(),
            Ok("from the process")
        );
        std::fs::remove_file(path).unwrap();
    }
}
