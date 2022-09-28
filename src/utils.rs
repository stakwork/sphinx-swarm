use bollard_stubs::models::{HostConfig, PortBinding, PortMap};

pub fn host_config(
    name: &str,
    ports: Vec<&str>,
    vols: Vec<&str>,
    links: Option<Vec<&str>>,
) -> Option<HostConfig> {
    let mut c = HostConfig {
        binds: volumes(name, vols),
        port_bindings: host_port(ports),
        ..Default::default()
    };
    if let Some(ls) = links {
        c.links = Some(ls.iter().map(|l| l.to_string()).collect());
    }
    Some(c)
}

fn volumes(name: &str, ins: Vec<&str>) -> Option<Vec<String>> {
    let pwd = std::env::current_dir().unwrap_or_default();
    let mut fulls: Vec<String> = Vec::new();
    for i in ins {
        let path = format!("{}/vol/{}:{}", pwd.to_string_lossy(), name, i);
        fulls.push(path);
    }
    println!("FULLS {:?}", fulls);
    Some(fulls)
}

fn host_port(ports_in: Vec<&str>) -> Option<PortMap> {
    let mut ports = PortMap::new();
    for port in ports_in {
        ports.insert(
            format!("{}/tcp", port).to_string(),
            Some(vec![PortBinding {
                host_port: Some(port.to_string()),
                host_ip: None,
            }]),
        );
    }
    Some(ports)
}
