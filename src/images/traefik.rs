use super::*;
use crate::utils::{domain, manual_host_config};
use bollard::container::Config;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraefikImage {
    pub name: String,
    pub insecure: bool,
    pub links: Links,
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
pub fn traefik(project: &str, img: &TraefikImage) -> Config<String> {
    let name = "traefik";
    let image = "traefik:v2.2.1";
    let root_vol = "traefik";
    let mut ports = vec!["8080", "443", "8883"];
    if img.insecure {
        ports.push("80");
    }
    let extra_vols = vec![
        "/var/run/docker.sock:/var/run/docker.sock",
        "/home/ec2-user/letsencrypt:/letsencrypt",
    ];
    let mut cmd = vec![
        "--providers.docker=true",
        "--providers.docker.exposedbydefault=false",
        "--entrypoints.web.address=:80",
        "--entrypoints.websecure.address=:443",
        // "--certificatesresolvers.myresolver.acme.email=evanfeenstra@gmail.com",
        // "--certificatesresolvers.myresolver.acme.storage=/letsencrypt/acme.json",
        // "--certificatesresolvers.myresolver.acme.caserver=https://acme-v02.api.letsencrypt.org/directory",
        "--certificatesresolvers.myresolver.acme.dnschallenge=true",
        "--certificatesresolvers.myresolver.acme.dnschallenge.provider=route53",
    ];
    if img.insecure {
        cmd.push("--log.level=DEBUG");
        cmd.push("--api.insecure=true");
    }
    let add_ulimits = true;
    let add_log_limit = true;
    Config {
        image: Some(image.to_string()),
        hostname: Some(domain(name)),
        host_config: manual_host_config(
            strarr(ports),
            Some(strarr(extra_vols)),
            Some(img.links.clone()),
            add_ulimits,
            add_log_limit,
        ),
        cmd: Some(strarr(cmd)),
        ..Default::default()
    }
}

pub fn traefik_labels(host: &str, port: &str) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    let lb = "traefik.http.services.elements.loadbalancer.server.port";
    let def = vec![
        "traefik.enable=true".to_string(),
        format!("traefik.http.routers.elements.rule=Host(`{}`)", host),
        format!("{}={}", lb, port),
        "traefik.http.routers.elements.tls=true".to_string(),
        "traefik.http.routers.elements.tls.certresolver=myresolver".to_string(),
        "traefik.http.routers.elements.entrypoints=websecure".to_string(),
    ];
    def.iter().for_each(|l| {
        let parts = l.split("=").collect::<Vec<&str>>();
        if parts.len() > 1 {
            labels.insert(parts[0].to_string(), parts[1].to_string());
        };
    });
    labels
}
