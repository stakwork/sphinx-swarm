use super::*;

use anyhow::{Context, Result};
use bollard::Docker;
use rocket::tokio;
use sphinx_swarm::config::{Clients, ExternalNodeType, Node};
use sphinx_swarm::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use sphinx_swarm::conn::proxy::ProxyAPI;
use sphinx_swarm::conn::relay::RelayAPI;
use sphinx_swarm::images::{Image, LinkedImages};
use sphinx_swarm::secrets;
use sphinx_swarm::{dock::*, images};
use std::collections::HashMap;
use url::{Host, Url};

pub async fn add_node(
    proj: &str,
    node: &Node,
    nodes: Vec<Node>,
    docker: &Docker,
    ids: &mut HashMap<String, String>,
    secs: &secrets::Secrets,
    clients: &mut Clients,
) -> Result<()> {
    if let Node::External(n) = node {
        log::info!("external url {}", n.url);
        return Ok(());
    }
    let node = node.as_internal().unwrap();
    match node {
        Image::Btc(btc) => {
            let btc1 = images::btc::btc(proj, &btc);
            let btc_id = create_and_start(&docker, btc1).await?;
            ids.insert(btc.name.clone(), btc_id);
            let client = BitcoinRPC::new(&btc, "http://127.0.0.1", "18443")?;
            sleep(1).await;
            client.create_or_load_wallet()?;
            clients.bitcoind.insert(btc.name, client);
            log::info!("created bitcoind");
        }
        Image::Lnd(lnd) => {
            // log::info!("wait 90 seconds...");
            sleep(1).await;
            let li = LinkedImages::from_nodes(lnd.links.clone(), nodes);
            let btc = li.find_btc().context("BTC required for LND")?;

            let lnd1 = images::lnd::lnd(proj, &lnd, &btc);
            let lnd_id = create_and_start(&docker, lnd1).await?;

            ids.insert(lnd.name.clone(), lnd_id.clone());

            // volume_permissions(proj, &lnd.name, "data")?;
            // let (client, test_mine_addy) = setup::lnd_clients(proj, &lnd, &secs, &lnd.name).await?;
            // setup::test_mine_if_needed(test_mine_addy, &btc.name, clients);
            // clients.lnd.insert(lnd.name, client);
            log::info!("created LND {}", lnd_id);
        }
        Image::Proxy(proxy) => {
            let li = LinkedImages::from_nodes(proxy.links.clone(), nodes);
            let lnd = li.find_lnd().context("LND required for Proxy")?;

            let proxy1 = images::proxy::proxy(proj, &proxy, &lnd);
            println!("creating proxy... {:?}", proxy1);
            let proxy_id = create_and_start(&docker, proxy1).await?;
            println!("created proxy...");
            ids.insert(proxy.name.clone(), proxy_id.clone());

            let client = ProxyAPI::new(&proxy).await?;
            clients.proxy.insert(proxy.name, client);

            log::info!("created Proxy {}", proxy_id);
        }
        Image::Relay(relay) => {
            let li = LinkedImages::from_nodes(relay.links.clone(), nodes);
            let lnd = li.find_lnd().context("LND required for Relay")?;
            let proxy = li.find_proxy();

            let relay1 = images::relay::relay(proj, &relay, &lnd, proxy);
            let relay_id = create_and_start(&docker, relay1).await?;
            ids.insert(relay.name.clone(), relay_id.clone());

            sleep(1).await;
            let client = RelayAPI::new(&relay, false).await?;
            // let client = relay_root_user(proj, &relay.name, client).await?;
            clients.relay.insert(relay.name, client);

            log::info!("created Relay {}", relay_id);
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

            let cache1 = images::cache::cache(proj, &cache, &memes_host, &tribe_host);
            let cache_id = create_and_start(&docker, cache1).await?;
            ids.insert(cache.name.clone(), cache_id);

            sleep(1).await;

            log::info!("created cache");
        }
    }
    Ok(())
}

async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}