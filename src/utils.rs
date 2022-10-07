use bollard_stubs::models::{HostConfig, PortBinding, PortMap};
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
        extra_hosts: Some(vec!["host.docker.internal:127.17.0.1".to_string()]),
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
