mod handler;
mod setup;
mod srv;

use anyhow::{Context, Result};
use bollard::Docker;
use rocket::tokio;
use sphinx_swarm::cmd::Cmd;
use sphinx_swarm::config::{
    load_config_file, put_config_file, Clients, ExternalNodeType, Node, Stack, State, STATE,
};
use sphinx_swarm::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use sphinx_swarm::conn::proxy::ProxyAPI;
use sphinx_swarm::conn::relay::RelayAPI;
use sphinx_swarm::images::{Image, LinkedImages};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::secrets;
use sphinx_swarm::{dock::*, images, logs};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use url::{Host, Url};

async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}

async fn add_node(
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
            let (client, test_mine_addy) = setup::lnd_clients(proj, &lnd, &secs, &lnd.name).await?;
            setup::test_mine_if_needed(test_mine_addy, &btc.name, clients);

            clients.lnd.insert(lnd.name, client);
            log::info!("created LND {}", lnd_id);
        }
        Image::Proxy(proxy) => {
            let li = LinkedImages::from_nodes(proxy.links.clone(), nodes);
            let lnd = li.find_lnd().context("LND required for Proxy")?;

            let proxy1 = images::proxy::proxy(proj, &proxy, &lnd);
            let proxy_id = create_and_start(&docker, proxy1).await?;
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

// return a map of name:docker_id
async fn build_stack(
    proj: &str,
    docker: &Docker,
    stack: &Stack,
) -> Result<(HashMap<String, String>, Clients)> {
    let secs = secrets::load_secrets(proj).await;
    let mut ids = HashMap::new();
    let mut clients: Clients = Default::default();
    for node in stack.nodes.clone().iter() {
        add_node(
            proj,
            &node,
            stack.nodes.clone(),
            docker,
            &mut ids,
            &secs,
            &mut clients,
        )
        .await?;
    }
    Ok((ids, clients))
}

#[rocket::main]
async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let proj = "stack";
    let stack: Stack = load_config_file(proj).await;
    let (ids, clients) = build_stack(proj, &docker, &stack).await?;
    put_config_file(proj, &stack).await;

    // set into the main state mutex
    let mut state = STATE.lock().await;
    *state = State { stack, clients };
    // drop it immediately
    drop(state);

    let (tx, mut rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    let docker2 = docker.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(cmd) = serde_json::from_str::<Cmd>(&msg.message) {
                match handler::handle(cmd, &msg.tag, &docker2).await {
                    Ok(res) => {
                        let _ = msg.reply_tx.send(res);
                    }
                    Err(err) => {
                        msg.reply_tx
                            .send(fmt_err(&err.to_string()))
                            .expect("couldnt send cmd reply");
                    }
                }
            } else {
                msg.reply_tx
                    .send(fmt_err("Invalid Command"))
                    .expect("couldnt send cmd reply");
            }
        }
    });

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = srv::launch_rocket(tx.clone(), log_txs).await?;

    // for (_, id) in ids {
    //     stop_and_remove(&docker, &id).await?;
    // }

    Ok(())
}

fn fmt_err(err: &str) -> String {
    format!("{{\"error\":\"{}\"}}", err.to_string())
}
