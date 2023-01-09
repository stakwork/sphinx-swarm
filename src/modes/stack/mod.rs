pub mod cmd;
mod handler;
mod srv;

use crate::config::{load_config_file, put_config_file, Clients, Node, Stack, State, STATE};
use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::conn::lnd::{lndrpc::LndRPC, unlocker::LndUnlocker};
use crate::conn::proxy::ProxyAPI;
use crate::conn::relay::RelayAPI;
use crate::images::Image;
use crate::rocket_utils::CmdRequest;
use crate::secrets;
use crate::{dock::*, images, logs};
use anyhow::{Context, Result};
use bollard::Docker;
use cmd::Cmd;
use core::time;
use images::{LndImage, ProxyImage};
use rocket::tokio;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use tokio::sync::{mpsc, Mutex};

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
    let node = node.as_internal()?;
    match node {
        Image::Btc(btc) => {
            let btc1 = images::btc(proj, &btc);
            let btc_id = create_and_start(&docker, btc1).await?;
            ids.insert(btc.name.clone(), btc_id);
            let client = BitcoinRPC::new(&btc, "http://127.0.0.1", "18443")?;
            clients.bitcoind.insert(btc.name, client);
            log::info!("created bitcoind");
        }
        Image::Lnd(lnd) => {
            // let delay_time = time::Duration::from_millis(90000);
            // thread::sleep(delay_time);

            let btc_name = lnd.links.get(0).context("LND requires a BTC")?;
            let btc = nodes
                .iter()
                .find(|n| &n.name() == btc_name)
                .context("No BTC found for LND")?
                .as_btc()?;
            let lnd1 = images::lnd(proj, &lnd, &btc);
            let lnd_id = create_and_start(&docker, lnd1).await?;

            ids.insert(lnd.name.clone(), lnd_id.clone());
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if let Err(e) = unlock_lnd(proj, &lnd, &secs, &lnd.name).await {
                log::error!("ERROR UNLOCKING LND {:?}", e);
            };
            // volume_permissions(proj, &lnd.name, "data")?;
            let client = LndRPC::new(proj, &lnd).await?;
            clients.lnd.insert(lnd.name, client);
            log::info!("created LND {}", lnd_id);
        }
        Image::Proxy(proxy) => {
            let lnd_name = proxy.links.get(0).context("Proxy requires a LND")?;
            let lnd = nodes
                .iter()
                .find(|n| &n.name() == lnd_name)
                .context("No LND found for Proxy")?
                .as_lnd()?;
            let proxy1 = images::proxy(proj, &proxy, &lnd);
            let proxy_id = create_and_start(&docker, proxy1).await?;
            ids.insert(proxy.name.clone(), proxy_id.clone());

            let client = ProxyAPI::new(&proxy).await?;
            clients.proxy.insert(proxy.name, client);

            log::info!("created Proxy {}", proxy_id);
        }
        Image::Relay(relay) => {
            let mut lnd: Option<&LndImage> = None;
            let mut proxy: Option<&ProxyImage> = None;
            relay.links.iter().for_each(|l| {
                if let Some(node) = nodes.iter().find(|n| &n.name() == l) {
                    match node {
                        Node::Internal(i) => match i {
                            Image::Proxy(p) => proxy = Some(p),
                            Image::Lnd(l) => lnd = Some(l),
                            _ => (),
                        },
                        Node::External(_e) => (),
                    }
                }
            });
            if let None = lnd {
                return Err(anyhow::anyhow!("LND required for Relay".to_string()));
            }
            let relay1 = images::relay(proj, &relay, lnd.unwrap(), proxy);
            let relay_id = create_and_start(&docker, relay1).await?;
            ids.insert(relay.name.clone(), relay_id.clone());

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let client = RelayAPI::new(&relay).await?;
            let client = relay_root_user(proj, &relay.name, client).await?;
            clients.relay.insert(relay.name, client);

            log::info!("created Relay {}", relay_id);
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

async fn relay_root_user(proj: &str, name: &str, api: RelayAPI) -> Result<RelayAPI> {
    let has_admin = api.has_admin().await?;
    if has_admin {
        log::info!("relay admin exists already");
        return Ok(api);
    }
    let new_user = api.add_user().await?;
    let token = secrets::random_word(12);
    let _id = api.claim_user(&new_user.public_key, &token).await?;
    secrets::add_to_secrets(proj, name, &token).await;
    Ok(api)
}

async fn unlock_lnd(
    proj: &str,
    lnd_node: &LndImage,
    secs: &secrets::Secrets,
    name: &str,
) -> Result<()> {
    // INIT LND
    let cert_path = format!("vol/{}/{}/tls.cert", proj, name);
    println!("Cert Path {}", cert_path);
    let unlock_port = lnd_node.http_port.clone().context("no unlock port")?;
    let unlocker = LndUnlocker::new(&unlock_port, &cert_path).await?;
    if let Some(_) = secs.get(&lnd_node.name) {
        let ur = unlocker.unlock_wallet(&lnd_node.unlock_password).await?;
        if let Some(err_msg) = ur.message {
            log::error!("FAILED TO UNLOCK LND {:?}", err_msg);
        } else {
            log::info!("LND WALLET UNLOCKED!");
        }
    } else {
        let seed = unlocker.gen_seed().await?;
        if let Some(msg) = seed.message {
            log::error!("gen seed error: {}", msg);
        }
        let mnemonic = seed.cipher_seed_mnemonic.expect("no mnemonic");
        let ir = unlocker
            .init_wallet(&lnd_node.unlock_password, mnemonic.clone())
            .await?;
        if let Some(err_msg) = ir.message {
            log::error!("FAILED TO INIT LND {:?}", err_msg);
        } else {
            log::info!("LND WALLET INITIALIZED!");
        }
        secrets::add_to_secrets(proj, &lnd_node.name, &mnemonic.clone().join(" ")).await;
    };
    Ok(())
}

pub async fn run(docker: Docker) -> Result<()> {
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
    let _r = srv::launch_rocket(tx.clone(), log_txs).await;

    for (_, id) in ids {
        remove_container(&docker, &id).await?;
    }

    Ok(())
}

fn fmt_err(err: &str) -> String {
    format!("{{\"error\":\"{}\"}}", err.to_string())
}
