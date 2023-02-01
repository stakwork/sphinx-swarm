use super::setup;
use anyhow::{Context, Result};
use bollard::Docker;
use rocket::tokio;
use sphinx_swarm::config::Stack;
use sphinx_swarm::config::{Clients, ExternalNodeType, Node};
use sphinx_swarm::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use sphinx_swarm::conn::proxy::ProxyAPI;
use sphinx_swarm::images::{Image, LinkedImages};
use sphinx_swarm::{dock::*, images};
use url::{Host, Url};

// return a map of name:docker_id
pub async fn build_stack(proj: &str, docker: &Docker, stack: &Stack) -> Result<Clients> {
    // first create the default network
    create_network(docker, None).await?;
    // then add the containers
    let mut clients: Clients = Default::default();
    let nodes = stack.nodes.clone();
    let only_node = std::env::var("ONLY_NODE").ok();
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
            let btc1 = images::btc::btc(&btc);
            if let Some(_) = create_and_start(&docker, btc1, skip).await? {
                match BitcoinRPC::new_and_create_wallet(&btc, "http://127.0.0.1", "18443").await {
                    Ok(client) => {
                        clients.bitcoind.insert(btc.name, client);
                    }
                    Err(e) => log::warn!("BitcoinRPC error: {:?}", e),
                };
            }
        }
        Image::Lnd(lnd) => {
            sleep(1).await;
            let li = LinkedImages::from_nodes(lnd.links.clone(), nodes);
            let btc = li.find_btc().context("BTC required for LND")?;
            let lnd1 = images::lnd::lnd(&lnd, &btc);
            if let Some(_) = create_and_start(&docker, lnd1, skip).await? {
                sleep(1).await;
                match setup::lnd_clients(docker, proj, &lnd).await {
                    Ok((client, test_mine_addy)) => {
                        setup::test_mine_if_needed(test_mine_addy, &btc.name, clients);
                        clients.lnd.insert(lnd.name, client);
                    }
                    Err(e) => log::warn!("lnd_clients error: {:?}", e),
                }
            }
        }
        Image::Proxy(proxy) => {
            let li = LinkedImages::from_nodes(proxy.links.clone(), nodes);
            let lnd = li.find_lnd().context("LND required for Proxy")?;
            let proxy1 = images::proxy::proxy(&proxy, &lnd);
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
            let li = LinkedImages::from_nodes(relay.links.clone(), nodes);
            let lnd = li.find_lnd().context("LND required for Relay")?;
            let proxy = li.find_proxy();
            let relay1 = images::relay::relay(&relay, &lnd, proxy);
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
            let memes = nodes
                .iter()
                .find(|n| n.is_ext_of_type(ExternalNodeType::Meme))
                .context("No Memes")?
                .as_external()?;

            let memes_url = Url::parse(format!("https://{}", memes.url).as_str())?;
            let memes_host = memes_url.host().unwrap_or(Host::Domain("")).to_string();

            let tribes = nodes
                .iter()
                .find(|n| n.is_ext_of_type(ExternalNodeType::Tribes))
                .context("No Tribes")?
                .as_external()?;

            let tribes_url = Url::parse(format!("https://{}", tribes.url).as_str())?;
            let tribe_host = tribes_url.host().unwrap_or(Host::Domain("")).to_string();

            let cache1 = images::cache::cache(&cache, &memes_host, &tribe_host);
            create_and_start(&docker, cache1, skip).await?;
        }
        Image::Traefik(traefik) => {
            sleep(1).await;
            let t1 = images::traefik::traefik(&traefik);
            create_and_start(&docker, t1, skip).await?;
        }
    })
}

async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
