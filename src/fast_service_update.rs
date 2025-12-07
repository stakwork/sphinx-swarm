use anyhow::Result;
use bollard::Docker;

use crate::{config::Node, dock::get_image_version};

pub async fn handle_fast_node_update(proj: &str, docker: &Docker, nodes: Vec<Node>) -> Result<()> {
    // get the supported services
    let supported_services: Vec<&str> = vec!["stakgraph", "repo2graph"];

    // loop through them
    for service in supported_services {
        // check if there is a new version
        let image_version = get_image_version(service, &nodes, &docker).await;
        log::info!("{}: {:#?}", service, image_version);

        if image_version.is_latest {
            log::info!("{} is up to date!!", service);
            return Ok(());
        }

        // if we don't have a new version exit and continue
        // if we have a new version check if service is busy
        // if service is busy exist
        // if service is up not busy update service!
    }
    Ok(())
}
