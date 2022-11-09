mod handler;
mod secrets;
mod srv;

use crate::conn::lnd::unlocker::LndUnlocker;
use crate::rocket_utils::CmdRequest;
use crate::{cmd::Cmd, dock::*, images, logs};
use anyhow::Result;
use bollard::Docker;
use images::{BtcImage, LndImage, ProxyImage, RelayImage};
use rocket::tokio;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub async fn run(docker: Docker) -> Result<()> {
    let proj = "stack";
    let network = "regtest";
    let secrets = secrets::load_secrets(proj);

    // BITCOIND
    let btc_node = BtcImage::new("bitcoind", network, "sphinx", &secrets.bitcoind_pass);
    let btc1 = images::btc(proj, &btc_node);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    // LND
    let http_port = "8881";
    let lnd_node = LndImage::new("lnd1", network, "10009");
    let lnd1 = images::lnd(proj, &lnd_node, &btc_node, Some(http_port));
    let lnd_id = create_and_start(&docker, lnd1).await?;
    log::info!("created LND");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // INIT LND
    let cert_path = "vol/stack/lnd1/tls.cert";
    let unlocker = LndUnlocker::new(http_port, cert_path).await?;
    if let Some(_) = secrets.lnd1_mnemonic {
        let _ = unlocker.unlock_wallet(&secrets.lnd1_password).await?;
        log::info!("LND WALLET UNLOCKED!");
    } else {
        let seed = unlocker.gen_seed().await?;
        if let Some(msg) = seed.message {
            log::error!("gen seed error: {}", msg);
        }
        let mnemonic = seed.cipher_seed_mnemonic.expect("no mnemonic");
        let _ = unlocker
            .init_wallet(&secrets.lnd1_password, mnemonic.clone())
            .await?;
        log::info!("LND WALLET INITIALIZED!");
        secrets::add_mnemonic_to_secrets(proj, mnemonic.clone());
    };

    // PROXY
    let token = secrets.proxy_admin_token;
    let storekey = secrets.proxy_store_key;
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

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(cmd) = serde_json::from_str::<Cmd>(&msg.message) {
                handler::handle(cmd, &msg.tag).await;
            } else {
                msg.reply_tx
                    .send("Invalid command".to_string())
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
