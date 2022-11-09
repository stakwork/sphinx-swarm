mod handler;
mod srv;

use crate::config::{load_config_file, put_config_file, Node, Stack};
use crate::conn::lnd::unlocker::LndUnlocker;
use crate::images::Image;
use crate::rocket_utils::CmdRequest;
use crate::secrets;
use crate::{cmd::Cmd, dock::*, images, logs};
use anyhow::{Context, Result};
use bollard::Docker;
use images::{BtcImage, LndImage, ProxyImage, RelayImage};
use rocket::tokio;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

async fn add_node(
    proj: &str,
    node: &Node,
    nodes: Vec<Node>,
    docker: &Docker,
    ids: &mut HashMap<String, String>,
    secs: &secrets::Secrets,
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
            ids.insert(btc.name, btc_id);
            log::info!("created bitcoind");
        }
        Image::Lnd(lnd) => {
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
            if let Err(e) = unlock_lnd(proj, &lnd, &secs).await {
                log::error!("ERROR UNLOCKING LND {:?}", e);
            };
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
            ids.insert(proxy.name, proxy_id.clone());
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
            let relay_node = RelayImage::new("relay1", "3000");
            let relay1 = images::relay(proj, &relay_node, lnd.unwrap(), proxy);
            let relay_id = create_and_start(&docker, relay1).await?;
            ids.insert(relay.name, relay_id.clone());
            log::info!("created Relay {}", relay_id);
        }
        _ => log::warn!("nodes iter invalid node type"),
    }
    Ok(())
}

// return a map of name:docker_id
async fn build_stack(
    proj: &str,
    docker: &Docker,
    stack: &Stack,
) -> Result<HashMap<String, String>> {
    let secs = secrets::load_secrets(proj).await;
    let mut ids = HashMap::new();
    for node in stack.nodes.clone().iter() {
        add_node(proj, &node, stack.nodes.clone(), docker, &mut ids, &secs).await?;
    }
    Ok(ids)
}

async fn unlock_lnd(proj: &str, lnd_node: &LndImage, secs: &secrets::Secrets) -> Result<()> {
    // INIT LND
    let cert_path = format!("vol/{}/lnd1/tls.cert", proj);
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
    let ids = build_stack(proj, &docker, &stack).await?;
    put_config_file(proj, &stack).await;

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

pub async fn run1(docker: Docker) -> Result<()> {
    let proj = "stack";
    let network = "regtest";
    let secs = secrets::load_secrets(proj).await;
    println!("SECRETS {:?}", secs);

    // BITCOIND
    let btc_node = BtcImage::new("bitcoind", network, "sphinx");
    let btc1 = images::btc(proj, &btc_node);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    // LND
    let lnd_http_port = "8881";
    let mut lnd_node = LndImage::new("lnd1", network, "10009");
    lnd_node.http_port = Some(lnd_http_port.to_string());
    if let Some(up) = secs.get("lnd1") {
        lnd_node.unlock_password(up);
    }
    let lnd1 = images::lnd(proj, &lnd_node, &btc_node);
    let lnd_id = create_and_start(&docker, lnd1).await?;
    log::info!("created LND");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    if let Err(e) = unlock_lnd(proj, &lnd_node, &secs).await {
        log::error!("ERR UNLOCKING LND {:?}", e);
    }

    // PROXY
    let mut proxy_node = ProxyImage::new("proxy1", network, "11111", "5050");
    proxy_node.new_nodes(Some("0".to_string()));
    let proxy1 = images::proxy(proj, &proxy_node, &lnd_node);
    let proxy_id = create_and_start(&docker, proxy1).await?;
    log::info!("created PROXY");

    // RELAY
    let relay_node = RelayImage::new("relay1", "3000");
    let relay1 = images::relay(proj, &relay_node, &lnd_node, Some(&proxy_node));
    let relay_id = create_and_start(&docker, relay1).await?;

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

    // shutdown containers
    remove_container(&docker, &btc_id).await?;
    remove_container(&docker, &lnd_id).await?;
    remove_container(&docker, &proxy_id).await?;
    remove_container(&docker, &relay_id).await?;

    Ok(())
}

fn fmt_err(err: &str) -> String {
    format!("{{\"error\":\"{}\"}}", err.to_string())
}
