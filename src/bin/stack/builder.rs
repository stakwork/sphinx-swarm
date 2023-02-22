use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use sphinx_swarm::cmd::UpdateNode;
use sphinx_swarm::config::{Clients, Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::dock::{create_and_start, stop_and_remove};
use sphinx_swarm::images::{DockerConfig, Image};

// return a map of name:docker_id
pub async fn build_stack(proj: &str, docker: &Docker, stack: &Stack) -> Result<Clients> {
    // first create the default network
    create_network(docker, None).await?;
    // then add the containers
    let mut clients: Clients = Default::default();
    let nodes = stack.nodes.clone();
    let mut only_node = std::env::var("ONLY_NODE").ok();
    if only_node == Some("".to_string()) {
        only_node = None;
    }
    for node in nodes.iter() {
        let skip = match &only_node {
            Some(only) => &node.name() != only,
            None => false,
        };
        if let Err(e) = add_node(proj, node, &nodes, docker, &mut clients, skip).await {
            log::error!("add_node failed: {:?}", e);
        };
    }
    Ok(clients)
}

pub async fn add_node(
    proj: &str,
    node: &Node,
    nodes: &Vec<Node>,
    docker: &Docker,
    clients: &mut Clients,
    skip: bool,
) -> Result<()> {
    if let Node::External(n) = node {
        log::info!("external url {}", n.url);
        return Ok(());
    }
    let img = node.as_internal().unwrap();
    // create config
    let node_config = img.make_config(nodes, docker).await?;
    // start container
    create_and_start(docker, node_config, skip).await?;
    // post-starup steps (LND unlock)
    img.post_startup(proj, docker, clients).await?;
    // create a connect client
    img.connect_client(proj, clients, docker, nodes).await?;
    Ok(())
}

pub fn find_image_by_hostname(nodes: &Vec<Node>, hostname: &str) -> Result<Image> {
    let name = hostname
        .strip_suffix(".sphinx")
        .context(format!("no {:?}", hostname))?;
    Ok(nodes
        .iter()
        .find(|n| n.name() == name)
        .context(format!("No {}", name))?
        .as_internal()?)
}

pub async fn update_node(docker: &Docker, un: &UpdateNode, nodes: &mut Vec<Node>) -> Result<()> {
    let pos = nodes.iter().position(|n| n.name() == un.id);
    let hostname = format!("{}.sphinx", &un.id);
    if let None = pos {
        return Err(anyhow!("cannot find node in stack"));
    }
    let pos = pos.unwrap();

    // stop the node
    stop_and_remove(docker, &hostname).await?;

    nodes[pos].set_version(&un.version)?;

    let theimg = nodes[pos].as_internal()?;
    let theconfig = theimg.make_config(nodes, docker).await?;

    create_and_start(docker, theconfig, false).await?;

    Ok(())
}
