use super::*;
use crate::utils::{domain, manual_host_config};
use bollard::container::Config;
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
pub fn traefik(project: &str, insecure: bool) -> Config<String> {
    let name = "traefik";
    let image = "traefik:v2.2.1";
    let root_vol = "traefik";
    let mut ports = vec!["8080", "443", "8883"];
    if insecure {
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
        "--certificatesresolvers.myresolver.acme.email=evanfeenstra@gmail.com",
        "--certificatesresolvers.myresolver.acme.storage=/letsencrypt/acme.json",
        // "--certificatesresolvers.myresolver.acme.caserver=https://acme-v02.api.letsencrypt.org/directory",
        "--certificatesresolvers.myresolver.acme.dnschallenge=true",
        "--certificatesresolvers.myresolver.acme.dnschallenge.provider=route53",
    ];
    if insecure {
        cmd.push("--log.level=DEBUG");
        cmd.push("--api.insecure=true");
    }
    // ?
    let links = None;
    let add_ulimits = true;
    let add_log_limit = true;
    Config {
        image: Some(image.to_string()),
        hostname: Some(domain(name)),
        host_config: manual_host_config(
            strarr(ports),
            Some(strarr(extra_vols)),
            links,
            add_ulimits,
            add_log_limit,
        ),
        cmd: Some(strarr(cmd)),
        ..Default::default()
    }
}
