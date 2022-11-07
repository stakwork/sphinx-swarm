mod handler;
mod secrets;
mod srv;

use crate::config::{Config, NodeKind};
use crate::conn::lnd::unlocker::LndUnlocker;
use crate::images::Image;
use crate::rocket_utils::CmdRequest;
use crate::{cmd::Cmd, dock::*, images, logs};
use anyhow::Result;
use bollard::Docker;
use images::{BtcImage, LndImage, ProxyImage, RelayImage};
use rocket::tokio;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

async fn add_node(
    proj: &str,
    node: &NodeKind,
    docker: &Docker,
    ids: &mut HashMap<String, String>,
) -> Result<()> {
    match node {
        NodeKind::External(url) => {
            // log::info!("external url {}", url);
        }
        NodeKind::Internal(n) => match n.image.clone() {
            Image::Btc(btc) => {
                let btc1 = images::btc(proj, &btc);
                let btc_id = create_and_start(&docker, btc1).await?;
                ids.insert(btc.name, btc_id);
                log::info!("created bitcoind");
            }
            Image::Lnd(lnd) => (),
            Image::Proxy(proxy) => (),
            Image::Relay(relay) => (),
            _ => log::warn!("nodes iter invalid node type"),
        },
    }
    Ok(())
}

// return a map of name:docker_id
async fn build_stack(docker: Docker, conf: Config) -> Result<HashMap<String, String>> {
    let proj = "stack";
    let mut ids = HashMap::new();
    for node in conf.nodes.iter() {
        add_node(proj, node, &docker, &mut ids).await?;
    }
    Ok(ids)
}

pub async fn run(docker: Docker) -> Result<()> {
    let proj = "stack";
    let network = "regtest";
    let secrets = secrets::load_secrets(proj);

    // BITCOIND
    let btc_node = BtcImage::new(
        "bitcoind",
        network,
        "sphinx",
        secrets.get("bitcoin_pass").unwrap(),
    );
    let btc1 = images::btc(proj, &btc_node);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    // LND
    let lnd_http_port = "8881";
    let mut lnd_node = LndImage::new("lnd1", network, "10009");
    lnd_node.http_port = Some(lnd_http_port.to_string());
    let lnd1 = images::lnd(proj, &lnd_node, &btc_node);
    let lnd_id = create_and_start(&docker, lnd1).await?;
    log::info!("created LND");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // INIT LND
    let cert_path = "vol/stack/lnd1/tls.cert";
    let unlocker = LndUnlocker::new(lnd_http_port, cert_path).await?;
    let lnd_pass = secrets.get("lnd1_password").unwrap();
    if let Some(_) = secrets.get("lnd1_mnemonic") {
        let _ = unlocker.unlock_wallet(lnd_pass).await?;
        log::info!("LND WALLET UNLOCKED!");
    } else {
        let seed = unlocker.gen_seed().await?;
        if let Some(msg) = seed.message {
            log::error!("gen seed error: {}", msg);
        }
        let mnemonic = seed.cipher_seed_mnemonic.expect("no mnemonic");
        let _ = unlocker.init_wallet(lnd_pass, mnemonic.clone()).await?;
        log::info!("LND WALLET INITIALIZED!");
        secrets::add_to_secrets(proj, "lnd1_mnemonic", &mnemonic.clone().join(" "));
    };

    // PROXY
    let token = secrets.get("proxy_admin_token").unwrap();
    let storekey = secrets.get("proxy_store_key").unwrap();
    let mut proxy_node = ProxyImage::new("proxy1", network, "11111", "5050", &token, &storekey);
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
