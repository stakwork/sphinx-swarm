use crate::config::{self, Clients, Node, Stack, State, STATE};
use crate::dock::*;
use crate::dock::{
    prune_images, pull_image, restart_container, restore_backup_if_exist, stop_and_remove,
};
use crate::images::{DockerConfig, Image};
use crate::utils::domain;
use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use std::sync::atomic::{AtomicBool, Ordering};

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub static AUTO_UPDATE: AtomicBool = AtomicBool::new(false);

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

use tokio_cron_scheduler::{Job, JobScheduler};
pub async fn auto_updater(
    proj: &str,
    docker: Docker,
    node_names: Vec<String>,
) -> Result<JobScheduler> {
    log::info!(":auto_updater");
    let sched = JobScheduler::new().await?;
    // every day at 2 am
    // 0 2 * * *
    // every 6 hours
    // 0 */6 * * *
    // every hour
    // 0 0 * * * *
    sched
        .add(Job::new_async("@daily", |_uuid, _l| {
            Box::pin(async move {
                if !AUTO_UPDATE.load(Ordering::Relaxed) {
                    AUTO_UPDATE.store(true, Ordering::Relaxed);
                }
            })
        })?)
        .await?;

    sched.start().await?;

    let proj = proj.to_string();
    // let node_names = node_names.clone();
    tokio::spawn(async move {
        let node_names_ = node_names.clone();
        loop {
            let go = AUTO_UPDATE.load(Ordering::Relaxed);
            if go {
                for nn in &node_names_ {
                    if let Err(e) = update_node_from_state(&proj, &docker, nn).await {
                        log::error!("{:?}", e);
                    }
                }
                AUTO_UPDATE.store(false, Ordering::Relaxed);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    Ok(sched)
}

pub async fn update_node_from_state(proj: &str, docker: &Docker, node_name: &str) -> Result<()> {
    let mut state = STATE.lock().await;
    let nodes = state.stack.nodes.clone();
    let img = find_img(node_name, &nodes)?;

    // drop(state);
    match update_node(proj, docker, node_name, &nodes, &img, &mut state.clients).await {
        Ok(()) => {
            // let mut state = STATE.lock().await;
            // FIXME if this never returns then STATE will deadlock
            // for example new CLN does spin up GRPC until remote signer is connected
            let oy = match make_client(proj, docker, &img, &mut state).await {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow!("FAILED TO MAKE CLIENT {:?}", e)),
            };
            drop(state);
            oy
        }
        Err(e) => Err(anyhow!("FAILED TO UPDATE NODE {:?}", e)),
    }
}

pub async fn update_node_and_make_client(
    proj: &str,
    docker: &Docker,
    node_name: &str,
    state: &mut State,
) -> Result<()> {
    let img = find_img(node_name, &state.stack.nodes)?;
    match update_node(
        proj,
        docker,
        node_name,
        &state.stack.nodes,
        &img,
        &mut state.clients,
    )
    .await
    {
        Ok(_) => {
            let oy = match make_client(proj, docker, &img, state).await {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow!("FAILED TO MAKE CLIENT {:?}", e)),
            };
            oy
        }
        Err(e) => Err(anyhow!("FAILED TO UPDATE NODE {:?}", e)),
    }
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
    let (new_id, need_to_start, created_new_volume) =
        create_and_init(docker, node_config, skip).await?;
    if need_to_start {
        let id = new_id.context("new container should have an id")?;
        if let Err(e) = img.pre_startup(docker).await {
            log::warn!("pre_startup failed {} {:?}", id, e);
        }
        start_container(docker, &id).await?;
        if created_new_volume {
            // download from s3 if it does not exist already, unzip and copy to volume
            let result = restore_backup_if_exist(docker, &node.name()).await;
            //restart container
            match result {
                Ok(status) => {
                    if status == true {
                        restart_container(&docker, &id).await?;
                    }
                }
                Err(error) => {
                    log::error!("Error restoring Backup: {}", error)
                }
            }
        }
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

pub fn find_img(node_name: &str, nodes: &Vec<Node>) -> Result<Image> {
    let pos = nodes.iter().position(|n| n.name() == node_name);
    if let None = pos {
        return Err(anyhow!("cannot find node in stack"));
    }
    let pos = pos.unwrap();

    let theimg = nodes[pos].as_internal()?;

    Ok(theimg)
}

pub async fn update_node(
    proj: &str,
    docker: &Docker,
    node_name: &str,
    nodes: &Vec<Node>,
    theimg: &Image,
    clients: &mut config::Clients,
) -> Result<()> {
    let hostname = domain(&node_name);

    let theconfig = theimg.make_config(&nodes, docker).await?;

    //prune all unused images
    prune_images(docker).await;

    let need_to_start = pull_image(docker, theconfig.clone()).await?;

    if need_to_start {
        //remove clinet
        theimg.remove_client(clients);

        // stop the node
        stop_and_remove(docker, &hostname).await?;

        let id = create_container(&docker, theconfig).await?;
        log::info!("=> created {}", &hostname);

        //remove client
        start_container(&docker, &id).await?;

        // post-startup steps (LND unlock)
        theimg.post_startup(proj, docker).await?;
    }

    // nodes[pos].set_version(&un.version)?;

    Ok(())
}

async fn make_client(proj: &str, docker: &Docker, theimg: &Image, state: &mut State) -> Result<()> {
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
