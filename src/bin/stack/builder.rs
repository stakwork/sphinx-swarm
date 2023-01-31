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
use std::collections::HashMap;
use url::{Host, Url};

// return a map of name:docker_id
pub async fn build_stack(
    proj: &str,
    docker: &Docker,
    stack: &Stack,
) -> Result<(HashMap<String, String>, Clients)> {
    // first create the default network
    create_network(docker, None).await?;
    // then add the containers
    let mut ids = HashMap::new();
    let mut clients: Clients = Default::default();
    for node in stack.nodes.clone().iter() {
        let id_opt = add_node(proj, &node, stack.nodes.clone(), docker, &mut clients).await?;
        if let Some(id) = id_opt {
            ids.insert(node.name(), id);
        }
    }
    Ok((ids, clients))
}

pub async fn add_node(
    proj: &str,
    node: &Node,
    nodes: Vec<Node>,
    docker: &Docker,
    clients: &mut Clients,
) -> Result<Option<String>> {
    if let Node::External(n) = node {
        log::info!("external url {}", n.url);
        return Ok(None);
    }
    let node = node.as_internal().unwrap();
    let id = match node {
        Image::Btc(btc) => {
            let btc1 = images::btc::btc(&btc);
            let btc_id = create_and_start(&docker, btc1).await?;
            let client = BitcoinRPC::new(&btc, "http://127.0.0.1", "18443")?;
            sleep(1).await;
            client.create_or_load_wallet()?;
            clients.bitcoind.insert(btc.name, client);
            btc_id
        }
        Image::Lnd(lnd) => {
            sleep(1).await;
            let li = LinkedImages::from_nodes(lnd.links.clone(), nodes);
            let btc = li.find_btc().context("BTC required for LND")?;

            let lnd1 = images::lnd::lnd(&lnd, &btc);
            let lnd_id = create_and_start(&docker, lnd1).await?;

            sleep(1).await;
            let (client, test_mine_addy) = setup::lnd_clients(docker, proj, &lnd).await?;
            setup::test_mine_if_needed(test_mine_addy, &btc.name, clients);
            clients.lnd.insert(lnd.name, client);
            lnd_id
        }
        Image::Proxy(proxy) => {
            let li = LinkedImages::from_nodes(proxy.links.clone(), nodes);
            let lnd = li.find_lnd().context("LND required for Proxy")?;

            let proxy1 = images::proxy::proxy(&proxy, &lnd);
            let proxy_id = create_and_start(&docker, proxy1).await?;

            let client = ProxyAPI::new(&proxy).await?;
            clients.proxy.insert(proxy.name, client);
            proxy_id
        }
        Image::Relay(relay) => {
            sleep(1).await;
            let li = LinkedImages::from_nodes(relay.links.clone(), nodes);
            let lnd = li.find_lnd().context("LND required for Relay")?;
            let proxy = li.find_proxy();

            let relay1 = images::relay::relay(&relay, &lnd, proxy);
            let relay_id = create_and_start(&docker, relay1).await?;

            let client = setup::relay_client(proj, &relay).await?;
            clients.relay.insert(relay.name, client);
            relay_id
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
            let cache_id = create_and_start(&docker, cache1).await?;

            cache_id
        }
        Image::Traefik(traefik) => {
            sleep(1).await;
            let t1 = images::traefik::traefik(&traefik);
            let tid = create_and_start(&docker, t1).await?;
            tid
        }
    };
    Ok(Some(id))
}

async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
