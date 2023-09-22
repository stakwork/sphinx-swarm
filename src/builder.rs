use crate::cmd::UpdateNode;
use crate::config::{Clients, Node, Stack, State};
use crate::dock::*;
use crate::dock::{create_and_start, stop_and_remove};
use crate::images::{DockerConfig, Image};
use crate::utils::domain;
use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::dock;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn is_shutdown() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

pub fn shutdown_now() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

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
        if is_shutdown() {
            break;
        }
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
    let (new_id, need_to_start) = create_and_init(docker, node_config, skip).await?;
    if need_to_start {
        let id = new_id.context("new container should have an id")?;
        if let Err(e) = img.pre_startup(docker).await {
            log::warn!("pre_startup failed {} {:?}", id, e);
        }
        start_container(docker, &id).await?;
    }
    // post-startup steps (LND unlock)
    img.post_startup(proj, docker).await?;
    // create and connect client
    img.connect_client(proj, clients, docker, nodes, is_shutdown)
        .await?;
    // post-client connection steps (BTC load wallet)
    img.post_client(clients).await?;
    Ok(())
}

pub fn find_image_by_hostname(nodes: &Vec<Node>, hostname: &str) -> Result<Image> {
    let name = hostname.strip_suffix(".sphinx").unwrap_or(hostname);
    Ok(nodes
        .iter()
        .find(|n| n.name() == name)
        .context(format!("No {}", name))?
        .as_internal()?)
}

pub async fn update_node(
    proj: &str,
    docker: &Docker,
    un: &UpdateNode,
    state: &mut State,
    // clients: &mut Clients,
) -> Result<()> {
    let pos = state.stack.nodes.iter().position(|n| n.name() == un.id);
    let hostname = domain(&un.id);
    if let None = pos {
        return Err(anyhow!("cannot find node in stack"));
    }
    let pos = pos.unwrap();

    // stop the node
    stop_and_remove(docker, &hostname).await?;

    state.stack.nodes[pos].set_version(&un.version)?;

    let theimg = state.stack.nodes[pos].as_internal()?;
    let theconfig = theimg.make_config(&state.stack.nodes, docker).await?;

    create_and_start(docker, theconfig, false).await?;

    // post-startup steps (LND unlock)
    theimg.post_startup(proj, docker).await?;
    // create and connect client
    theimg
        .connect_client(
            proj,
            &mut state.clients,
            docker,
            &state.stack.nodes,
            is_shutdown,
        )
        .await?;
    // post-client connection steps (BTC load wallet)
    theimg.post_client(&mut state.clients).await?;

    Ok(())
}

pub async fn update_image(
    proj: &str,
    docker: &Docker,
    state: &mut State,
    // clients: &mut Clients,
) -> Result<()> {

    let mut image_list: Vec<String> = Vec::new();

    let containers = dock::list_containers(&docker).await?;

    for container in containers {
        if let Some(image) = container.image {
            if is_sphinx_image(&image) {
                println!(">>>>>><<<<<2 {:?}", &image);
                if let Some(image_name) = image.split(':').next() {
                    // remove_image
                    remove_image(docker, image_name).await?;
                    image_list.push(image_name.to_string());
                }
            }
        } else {
            println!("None Image");
        }
    }
    println!(">>>>>><<<<<3 {:?}", image_list);

    for image in image_list {
        let un = UpdateNode {
            id: image,
            version: "latest".to_owned(),
        };
        update_node(proj, &docker, &un, state);
    }


    Ok(())
}


fn is_sphinx_image(img_tag: &str) -> bool {
    img_tag.contains("sphinx-")
        || img_tag.contains("-sphinx")
        || img_tag.contains("cln-htlc-interceptor")
}
