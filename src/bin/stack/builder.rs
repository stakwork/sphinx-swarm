use super::setup;
use anyhow::{Context, Result};
use bollard::Docker;
use rocket::tokio;
use sphinx_swarm::config::Stack;
use sphinx_swarm::config::{Clients, Node};
use sphinx_swarm::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use sphinx_swarm::conn::proxy::ProxyAPI;
use sphinx_swarm::dock::*;
use sphinx_swarm::images::{DockerConfig, Image, LinkedImages};
use sphinx_swarm::utils::docker_domain_127;

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
    let node = node.as_internal().unwrap();
    Ok(match node {
        Image::Btc(btc) => {
            let btc1 = btc.make_config(nodes, docker).await?;
            if let Some(_) = create_and_start(&docker, btc1, skip).await? {
                let btc_rpc_url = format!("http://{}", docker_domain_127(&btc.name));
                match BitcoinRPC::new_and_create_wallet(&btc, &btc_rpc_url, "18443").await {
                    Ok(client) => {
                        clients.bitcoind.insert(btc.name, client);
                    }
                    Err(e) => log::warn!("BitcoinRPC error: {:?}", e),
                };
            }
        }
        Image::Lnd(lnd) => {
            sleep(1).await;
            let lnd1 = lnd.make_config(nodes, docker).await?;
            if let Some(_) = create_and_start(&docker, lnd1, skip).await? {
                sleep(1).await;
                match setup::lnd_clients(docker, proj, &lnd).await {
                    Ok((client, test_mine_addy)) => {
                        let li = LinkedImages::from_nodes(lnd.links.clone(), nodes);
                        let btc = li.find_btc().context("BTC required for LND")?;
                        setup::test_mine_if_needed(test_mine_addy, &btc.name, clients);
                        clients.lnd.insert(lnd.name, client);
                    }
                    Err(e) => log::warn!("lnd_clients error: {:?}", e),
                }
            }
        }
        Image::Proxy(proxy) => {
            let proxy1 = proxy.make_config(nodes, docker).await?;
            if let Some(_) = create_and_start(&docker, proxy1, skip).await? {
                match ProxyAPI::new(&proxy).await {
                    Ok(client) => {
                        clients.proxy.insert(proxy.name, client);
                    }
                    Err(e) => log::warn!("ProxyAPI error: {:?}", e),
                }
            }
        }
        Image::Relay(relay) => {
            sleep(1).await;
            let relay1 = relay.make_config(nodes, docker).await?;
            if let Some(_) = create_and_start(&docker, relay1, skip).await? {
                match setup::relay_client(proj, &relay).await {
                    Ok(client) => {
                        clients.relay.insert(relay.name, client);
                    }
                    Err(e) => log::warn!("relay_client error: {:?}", e),
                }
            }
        }
        Image::Cache(cache) => {
            let cache1 = cache.make_config(nodes, docker).await?;
            create_and_start(&docker, cache1, skip).await?;
        }
        Image::Neo4j(neo4j) => {
            let neo = neo4j.make_config(nodes, docker).await?;
            create_and_start(&docker, neo, skip).await?;
        }
        Image::Jarvis(jarvis) => {
            let j = jarvis.make_config(nodes, docker).await?;
            create_and_start(&docker, j, skip).await?;
        }
        Image::BoltWall(boltwall) => {
            let b = boltwall.make_config(nodes, docker).await?;
            create_and_start(&docker, b, skip).await?;
        }
        Image::NavFiber(navfiber) => {
            sleep(1).await;
            let nf = navfiber.make_config(nodes, docker).await?;
            create_and_start(&docker, nf, skip).await?;
        }
    })
}

async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
