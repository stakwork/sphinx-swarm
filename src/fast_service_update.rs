use crate::images::DockerHubImage;
use crate::utils::docker_domain;
use anyhow::Result;
use bollard::Docker;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::builder::{find_img, update_node_from_state};
use crate::{config::Node, dock::get_image_version};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BusyResponse {
    busy: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SupportedService {
    name: String,
    port: String,
}

fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build fast node reqwest client")
}

pub async fn handle_fast_node_update(proj: &str, docker: &Docker, nodes: Vec<Node>) -> Result<()> {
    // get the supported services
    let supported_services: Vec<SupportedService> = vec![
        SupportedService {
            name: "stakgraph".to_string(),
            port: "7799".to_string(), // TODO: find a better way to access the node port
        },
        SupportedService {
            name: "repo2graph".to_string(),
            port: "3355".to_string(),
        },
    ];

    // loop through them
    for service in supported_services {
        let img = find_img(&service.name, &nodes)?;
        let image_version = get_image_version(&service.name, docker, &img.repo().org).await;

        if image_version.is_latest {
            log::info!("{} is up to date!!", service.name);
            continue;
        }

        let service_status_response = get_service_busy_status(&service.name, &service.port).await;

        if service_status_response {
            log::info!("{} is busy at the moment", service.name);
            continue;
        }

        if let Err(e) = update_node_from_state(proj, docker, &service.name).await {
            log::error!("{:?}", e);
            // TODO: We should implement incident so we know when an error occurs
        }
    }
    Ok(())
}

pub async fn get_service_busy_status(name: &str, port: &str) -> bool {
    let client = make_client();
    let host = docker_domain(name);
    let route = format!("http://{}:{}/busy", host, port);

    match client.get(route.as_str()).send().await {
        Ok(res) => match res.json::<BusyResponse>().await {
            Ok(body) => {
                return body.busy;
            }
            Err(err) => {
                log::error!("Error parsing {} busy endpoint: {}", name, err.to_string());
                return true;
            }
        },
        Err(err) => {
            log::error!("Error calling {} busy endpoint: {}", name, err.to_string());
            return true;
        }
    }
}
