use super::*;
use crate::utils::{domain, getenv, manual_host_config};
use bollard::container::Config;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TraefikImage {
    pub name: String,
    pub links: Links,
}

impl TraefikImage {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            links: vec![],
        }
    }
    pub fn links(&mut self, links: Vec<&str>) {
        self.links = strarr(links)
    }
}
impl DockerHubImage for TraefikImage {
    fn repo(&self) -> Repository {
        Repository {
            registry: Registry::DockerHub,
            org: "library".to_string(),
            repo: "traefik".to_string(),
            root_volume: "/data".to_string(),
        }
    }
}

/*
environment:
      - AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID
      - AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY
      - AWS_REGION=$AWS_REGION

logging:
      options:
        max-size: 10m

ulimits:
      nproc: 65535
      nofile:
        soft: 1000000
        hard: 1000000
*/

fn _aws_env() -> Option<Vec<String>> {
    let aws_region = std::env::var("AWS_REGION");
    if let Err(_) = aws_region {
        return None;
    }
    Some(vec![format!("AWS_REGION={}", aws_region.unwrap())])
}

fn _traefik(img: &TraefikImage) -> Config<String> {
    let name = img.name.clone();
    let image = "traefik:v2.9";
    let mut ports = vec!["80", "443"];
    let insecure = match std::env::var("TRAEFIK_INSECURE") {
        Ok(_) => true,
        Err(_) => false,
    };
    // for the web dashboard
    if insecure {
        ports.push("8080");
    }
    let extra_vols = vec![
        "/var/run/docker.sock:/var/run/docker.sock",
        "/home/admin/letsencrypt:/letsencrypt",
    ];
    let mut cmd = vec![
        "--providers.docker=true",
        "--providers.docker.exposedbydefault=false",
        "--entrypoints.web.address=:80",
        "--entrypoints.websecure.address=:443",
        "--certificatesresolvers.myresolver.acme.email=evanfeenstra@gmail.com",
        "--certificatesresolvers.myresolver.acme.storage=/letsencrypt/acme.json",
        "--certificatesresolvers.myresolver.acme.dnschallenge=true",
        "--certificatesresolvers.myresolver.acme.dnschallenge.provider=route53",
    ];
    if let Ok(_) = std::env::var("TRAEFIK_STAGING") {
        // when u turn off testing, delete the certs in /home/admin/letsencrypt
        // cmd.push("--certificatesresolvers.myresolver.acme.caserver=https://acme-v02.api.letsencrypt.org/directory");
        cmd.push("--certificatesresolvers.myresolver.acme.caserver=https://acme-staging-v02.api.letsencrypt.org/directory");
        log::info!("traefik: testing ca server");
    } else {
        log::info!("traefik: prod ca server");
    }
    if insecure {
        cmd.push("--log.level=DEBUG");
        cmd.push("--api.insecure=true");
    }
    let add_ulimits = true;
    let add_log_limit = true;
    let awsenv = _aws_env();
    if let Some(ae) = &awsenv {
        log::info!("traefik: using AWS REGION env {:?}", ae.get(0));
    } else {
        log::error!("traefik: MISSING AWS REGION ENV!");
    }

    log::error!("traefik: MISSING AWS ENV!");

    Config {
        image: Some(image.to_string()),
        hostname: Some(domain(&name)),
        host_config: manual_host_config(
            strarr(ports),
            Some(strarr(extra_vols)),
            add_ulimits,
            add_log_limit,
        ),
        env: awsenv,
        cmd: Some(strarr(cmd)),
        ..Default::default()
    }
}

pub fn traefik_labels(
    name: &str,
    host: &str, // stakgraph.swarm38.sphinx.chat
    port: &str, // inner port (like 80 for navfiber nginx, not 8000)
    websockets: bool,
) -> HashMap<String, String> {
    if let Ok(pbs) = getenv("PORT_BASED_SSL") {
        if pbs == "true" || pbs == "1" {
            // use port based SSL
            return traefik_labels_port_based_ssl(name, host, port, websockets);
        }
    }
    let lb = format!("traefik.http.services.{}.loadbalancer.server.port", name);
    let mut def = vec![
        "traefik.enable=true".to_string(),
        format!("{}={}", lb, port),
        format!("traefik.http.routers.{}.tls=true", name),
        format!("traefik.http.routers.{}.tls.certresolver=myresolver", name),
        format!("traefik.http.routers.{}.entrypoints=websecure", name),
    ];
    if navfiber_boltwall_shared_host().is_some() && is_navfiber_or_boltwall(name) {
        let shared_host = navfiber_boltwall_shared_host().unwrap();
        if name == "navfiber" {
            // anything except /api (local resources)
            def.push(format!(
                "traefik.http.routers.{}.rule=Host(`{}`)",
                name, shared_host
            ));
            def.push(format!("traefik.http.routers.{}.priority=1", name));
        } else {
            // if /api then all should go here
            def.push(format!(
                "traefik.http.routers.{}.rule=Host(`{}`) && (PathPrefix(`/api`) || PathPrefix(`/socket.io`))",
                name, shared_host
            ));
            def.push(format!("traefik.http.routers.{}.priority=2", name));
        }
    } else {
        def.push(format!(
            "traefik.http.routers.{}.rule=Host(`{}`)",
            name, host
        ));
    }
    if websockets {
        def.push("traefik.http.middlewares.sslheader.headers.customrequestheaders.X-Forwarded-Proto=https".to_string())
    }
    to_labels(def)
}

pub fn extract_base_domain(host: &str) -> String {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 4 {
        // Skip the first part (subdomain) and join the rest
        parts[1..].join(".")
    } else {
        // If it's already a base domain or malformed, return as-is
        host.to_string()
    }
}

pub fn traefik_labels_port_based_ssl(
    name: &str,
    host: &str,
    port: &str,
    websockets: bool,
) -> HashMap<String, String> {
    let base_host = extract_base_domain(host);

    // SPECIAL case for navfiber (internal port 80 -> external port 8000)
    let entrypoint_port = if port == "80" { "8000" } else { port };
    let entrypoint_name = format!("port{}", entrypoint_port);
    let lb = format!("traefik.http.services.{}.loadbalancer.server.port", name);

    let mut def = vec![
        "traefik.enable=true".to_string(),
        format!("{}={}", lb, port),
        format!(
            "traefik.http.routers.{}.entrypoints={}",
            name, entrypoint_name
        ),
        format!("traefik.http.routers.{}.rule=Host(`{}`)", name, base_host), // Uses base domain
        // SSL configuration
        format!("traefik.http.routers.{}.tls=true", name),
        // format!("traefik.http.routers.{}.tls.certresolver=myresolver", name),
    ];

    if websockets {
        def.push("traefik.http.middlewares.sslheader.headers.customrequestheaders.X-Forwarded-Proto=https".to_string());
    }

    to_labels(def)
}

fn is_navfiber_or_boltwall(name: &str) -> bool {
    name == "navfiber" || name == "boltwall"
}
pub fn navfiber_boltwall_shared_host() -> Option<String> {
    let sh = std::env::var("NAV_BOLTWALL_SHARED_HOST").ok();
    match sh {
        Some(h) => {
            // remove empty string
            if h.len() > 0 {
                Some(h)
            } else {
                None
            }
        }
        None => None,
    }
}

pub fn cln_traefik_labels(
    name: &str,
    host: &str,
    peer_port: &str,
    ctrl_port: &str,
    mqtt_port: &str,
) -> HashMap<String, String> {
    let ctrl_name = format!("{}-ctrl", name);
    let ctrl_host = format!("ctrl-{}", host);
    let mqtt_name = format!("{}-mqtt", name);
    let mqtt_host = format!("mqtt-{}", host);
    let def = vec![
        "traefik.enable=true".to_string(),
        // main service (peering)
        format!("traefik.http.routers.{}.service={}", name, name),
        format!(
            "traefik.http.services.{}.loadbalancer.server.port={}",
            name, peer_port
        ),
        format!("traefik.http.routers.{}.rule=Host(`{}`)", name, host),
        format!("traefik.http.routers.{}.tls=true", name),
        format!("traefik.http.routers.{}.tls.certresolver=myresolver", name),
        format!("traefik.http.routers.{}.entrypoints=websecure", name),
        // ctrl service
        format!("traefik.http.routers.{}.service={}", ctrl_name, ctrl_name),
        format!(
            "traefik.http.services.{}.loadbalancer.server.port={}",
            ctrl_name, ctrl_port
        ),
        format!(
            "traefik.http.routers.{}.rule=Host(`{}`)",
            ctrl_name, ctrl_host
        ),
        format!("traefik.http.routers.{}.tls=true", ctrl_name),
        format!(
            "traefik.http.routers.{}.tls.certresolver=myresolver",
            ctrl_name
        ),
        format!("traefik.http.routers.{}.entrypoints=websecure", ctrl_name),
        // mqtt service (HostSNI and mqttsecure entrypoint)
        format!("traefik.tcp.routers.{}.service={}", mqtt_name, mqtt_name),
        format!(
            "traefik.tcp.services.{}.loadbalancer.server.port={}",
            mqtt_name, mqtt_port
        ),
        format!(
            "traefik.tcp.routers.{}.rule=HostSNI(`{}`)",
            mqtt_name, mqtt_host
        ),
        format!("traefik.tcp.routers.{}.tls=true", mqtt_name),
        format!(
            "traefik.tcp.routers.{}.tls.certresolver=myresolver",
            mqtt_name
        ),
        format!("traefik.tcp.routers.{}.entrypoints=mqttsecure", mqtt_name),
    ];
    to_labels(def)
}

pub fn broker_traefik_labels(
    name: &str,
    host: &str,
    mqtt_port: &str,
    ws_port: Option<&str>,
) -> HashMap<String, String> {
    let mqtt_name = format!("{}-mqtt", name);
    let mqtt_host = format!("mqtt-{}", host);
    let mut def = vec![
        "traefik.enable=true".to_string(),
        // mqtt service (HostSNI and mqttsecure entrypoint)
        format!("traefik.tcp.routers.{}.service={}", mqtt_name, mqtt_name),
        format!(
            "traefik.tcp.services.{}.loadbalancer.server.port={}",
            mqtt_name, mqtt_port
        ),
        format!(
            "traefik.tcp.routers.{}.rule=HostSNI(`{}`)",
            mqtt_name, mqtt_host
        ),
        format!("traefik.tcp.routers.{}.tls=true", mqtt_name),
        format!(
            "traefik.tcp.routers.{}.tls.certresolver=myresolver",
            mqtt_name
        ),
        format!("traefik.tcp.routers.{}.entrypoints=mqttsecure", mqtt_name),
    ];
    // ctrl service
    if let Some(wsp) = ws_port {
        let ws_name = name;
        let ws_host = host;
        let more = vec![
            format!("traefik.http.routers.{}.service={}", ws_name, ws_host),
            format!(
                "traefik.http.services.{}.loadbalancer.server.port={}",
                ws_name, wsp
            ),
            format!("traefik.http.routers.{}.rule=Host(`{}`)", ws_name, ws_host),
            format!("traefik.http.routers.{}.tls=true", ws_name),
            format!(
                "traefik.http.routers.{}.tls.certresolver=myresolver",
                ws_name
            ),
            format!("traefik.http.routers.{}.entrypoints=websecure", ws_name),
        ];
        def.extend_from_slice(&more);
    }
    to_labels(def)
}

pub fn neo4j_labels(
    name: &str,
    host: &str,
    http_port: &str,
    bolt_port: &str,
) -> HashMap<String, String> {
    let auth_user = "neo4j:test";
    let def = vec![
        "traefik.enable=true".to_string(),
        //
        format!("traefik.http.routers.{}.rule=Host(`{}`) && PathPrefix(`/neo4j`)", name, host),
        format!("traefik.http.routers.{}.tls=true", name),
        format!("traefik.http.routers.{}.entrypoints=websecure", name),
        format!("traefik.http.routers.{}.tls.certresolver=myresolver", name),
        format!("traefik.http.routers.{}.service={}", name, name),
        format!("traefik.http.routers.{}.middlewares=neo4j-auth,neo4j-prefix", name),
        format!("traefik.http.services.{}.loadbalancer.server.port={}", name, http_port),
        format!("traefik.http.middlewares.neo4j-auth.basicauth.users={}", auth_user),
        format!("traefik.http.middlewares.neo4j-prefix.stripprefix.prefixes=/neo4j"),
        //
        format!("traefik.http.routers.{}-bolt.rule=Host(`{}`)", name, host),
        format!("traefik.http.routers.{}-bolt.tls=true", name),
        format!("traefik.http.routers.{}-bolt.entrypoints=websecure", name),
        format!("traefik.http.routers.{}-bolt.tls.certresolver=myresolver", name),
        format!("traefik.http.routers.{}-bolt.service={}-bolt", name, name),
        format!("traefik.http.services.{}-bolt.loadbalancer.server.port={}", name, bolt_port),
        format!("traefik.http.middlewares.sslheader2.headers.customrequestheaders.X-Forwarded-Proto=https,wss"),
        format!("traefik.http.routers.{}-bolt.middlewares=sslheader2", name),
        //
        format!("traefik.tcp.routers.{}-bolt.rule=HostSNI(`{}`)", name, host),
        format!("traefik.tcp.routers.{}-bolt.tls=true", name),
        format!("traefik.tcp.routers.{}-bolt.entrypoints=websecure", name),
        format!("traefik.tcp.routers.{}-bolt.tls.certresolver=myresolver", name),
        format!("traefik.tcp.routers.{}-bolt.service={}-bolt", name, name),
        format!("traefik.tcp.services.{}-bolt.loadbalancer.server.port={}", name, bolt_port),
    ];
    to_labels(def)
}

pub fn elastic_labels(name: &str, host: &str, http_port: &str) -> HashMap<String, String> {
    let auth_user = "elastic:test";
    let def = vec![
        "traefik.enable=true".to_string(),
        //
        format!(
            "traefik.http.routers.{}.rule=Host(`{}`) && PathPrefix(`/elastic`)",
            name, host
        ),
        format!("traefik.http.routers.{}.tls=true", name),
        format!("traefik.http.routers.{}.entrypoints=websecure", name),
        format!("traefik.http.routers.{}.tls.certresolver=myresolver", name),
        format!("traefik.http.routers.{}.service={}", name, name),
        format!(
            "traefik.http.routers.{}.middlewares=elastic-auth,elastic-prefix",
            name
        ),
        format!(
            "traefik.http.services.{}.loadbalancer.server.port={}",
            name, http_port
        ),
        format!(
            "traefik.http.middlewares.elastic-auth.basicauth.users={}",
            auth_user
        ),
        format!("traefik.http.middlewares.elastic-prefix.stripprefix.prefixes=/elastic"),
    ];
    to_labels(def)
}

fn to_labels(def: Vec<String>) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    def.iter().for_each(|l| {
        let parts = l.split("=").collect::<Vec<&str>>();
        if parts.len() > 1 {
            labels.insert(parts[0].to_string(), parts[1].to_string());
        };
    });
    labels
}
