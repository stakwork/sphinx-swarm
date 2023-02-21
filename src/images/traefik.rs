use super::*;
use crate::utils::{domain, manual_host_config};
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
            org: "".to_string(),
            repo: "traefik".to_string(),
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

// all 3 required for inclusion
fn aws_env() -> Option<Vec<String>> {
    let aws_key = std::env::var("AWS_ACCESS_KEY_ID");
    let aws_secret = std::env::var("AWS_SECRET_ACCESS_KEY");
    let aws_region = std::env::var("AWS_REGION");
    if let Err(_) = aws_key {
        return None;
    }
    if let Err(_) = aws_secret {
        return None;
    }
    if let Err(_) = aws_region {
        return None;
    }
    Some(vec![
        format!("AWS_REGION={}", aws_region.unwrap()),
        format!("AWS_ACCESS_KEY_ID={}", aws_key.unwrap()),
        format!("AWS_SECRET_ACCESS_KEY={}", aws_secret.unwrap()),
    ])
}

pub fn traefik(img: &TraefikImage) -> Config<String> {
    let name = img.name.clone();
    let image = "traefik:v2.2.1";
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
    let awsenv = aws_env();
    if let Some(ae) = &awsenv {
        log::info!("traefik: using AWS env {:?}", ae.get(0));
    } else {
        log::error!("traefik: MISSING AWS ENV!");
    }
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
    host: &str,
    port: &str,
    websockets: bool,
) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    let lb = format!("traefik.http.services.{}.loadbalancer.server.port", name);
    let mut def = vec![
        "traefik.enable=true".to_string(),
        format!("{}={}", lb, port),
        format!("traefik.http.routers.{}.rule=Host(`{}`)", name, host),
        format!("traefik.http.routers.{}.tls=true", name),
        format!("traefik.http.routers.{}.tls.certresolver=myresolver", name),
        format!("traefik.http.routers.{}.entrypoints=websecure", name),
    ];
    if websockets {
        def.push("traefik.http.middlewares.sslheader.headers.customrequestheaders.X-Forwarded-Proto=https".to_string())
    }
    def.iter().for_each(|l| {
        let parts = l.split("=").collect::<Vec<&str>>();
        if parts.len() > 1 {
            labels.insert(parts[0].to_string(), parts[1].to_string());
        };
    });
    labels
}
