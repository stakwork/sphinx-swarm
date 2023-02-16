use super::setup;
use anyhow::{Context, Result};
use bollard::Docker;
use rocket::tokio;
use sphinx_swarm::config::Stack;
use sphinx_swarm::config::{Clients, ExternalNodeType, Node};
use sphinx_swarm::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use sphinx_swarm::conn::lnd::utils::{dl_cert, dl_macaroon, strip_pem_prefix_suffix};
use sphinx_swarm::conn::proxy::ProxyAPI;
use sphinx_swarm::images::lnd::to_lnd_network;
use sphinx_swarm::images::{Image, LinkedImages};
use sphinx_swarm::utils::docker_domain_127;
use sphinx_swarm::{dock::*, images};
use url::{Host, Url};

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
            let btc1 = images::btc::btc(&btc);
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
        Image::Neo4j(neo4j) => {
            let neo = images::neo4j::neo4j(&neo4j);
            create_and_start(&docker, neo, skip).await?;
        }
        Image::Jarvis(jarvis) => {
            let neo4j_node = nodes
                .iter()
                .find(|n| n.name() == "neo4j")
                .context("No Neo4j")?
                .as_internal()?
                .as_neo4j()?;
            let j = images::jarvis::jarvis(&jarvis, &neo4j_node);
            create_and_start(&docker, j, skip).await?;
        }
        Image::BoltWall(boltwall) => {
            let lnd_node = nodes
                .iter()
                .find(|n| n.name() == "lnd")
                .context("No LND")?
                .as_internal()?
                .as_lnd()?;

            let cert_path = "/home/.lnd/tls.cert";
            let cert_full = dl_cert(docker, &lnd_node.name, cert_path).await?;
            let cert64 = strip_pem_prefix_suffix(&cert_full);
            let netwk = to_lnd_network(lnd_node.network.as_str());
            let macpath = format!("/home/.lnd/data/chain/bitcoin/{}/admin.macaroon", netwk);
            let mac = dl_macaroon(docker, &lnd_node.name, &macpath).await?;

            let jarvis_node = nodes
                .iter()
                .find(|n| n.name() == "jarvis")
                .context("No Jarvis")?
                .as_internal()?
                .as_jarvis()?;

            let b = images::boltwall::boltwall(&boltwall, &mac, &cert64, &lnd_node, &jarvis_node);
            create_and_start(&docker, b, skip).await?;
        }
        Image::NavFiber(navfiber) => {
            sleep(1).await;
            let nf = images::navfiber::navfiber(&navfiber);
            create_and_start(&docker, nf, skip).await?;
        }
    })
}

async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
