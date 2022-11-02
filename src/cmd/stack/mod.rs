mod secrets;
mod srv;

use crate::grpc::lnd::unlocker::LndUnlocker;
use crate::rocket_utils::CmdRequest;
use crate::{dock::*, images, logs};
use anyhow::Result;
use bollard::Docker;
use rocket::tokio;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub async fn run(docker: Docker) -> Result<()> {
    let proj = "stack";
    let network = "regtest";
    let secrets = secrets::load_secrets(proj);

    // btc setup
    let btc_node = images::BtcNode::new("bitcoind", network, "sphinx", &secrets.bitcoind_pass);
    let btc1 = images::btc(proj, &btc_node);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    // lnd setup
    let http_port = "8881";
    let lnd_node = images::LndNode::new("lnd1", network, "10009", "/root/.lnd");
    let lnd1 = images::lnd(proj, &lnd_node, &btc_node, Some(http_port));
    let lnd_id = create_and_start(&docker, lnd1).await?;
    log::info!("created LND");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let cert_path = "vol/stack/lnd1/tls.cert";
    let unlocker = LndUnlocker::new(http_port, cert_path).await?;
    let res = unlocker
        .init_wallet(&secrets.lnd1_password, secrets.lnd1_mnemonic)
        .await?;
    log::info!("RES {:?}", res);
    let _ = unlocker.unlock_wallet(&secrets.lnd1_password).await?;

    let proxy_node = images::ProxyNode::new(
        "proxy1",
        network,
        "11111",
        "/proxy",
        "5050",
        &secrets.proxy_admin_token,
        &secrets.proxy_store_key,
    );
    let proxy1 = images::proxy(proj, &proxy_node, &lnd_node);
    let proxy_id = create_and_start(&docker, proxy1).await?;
    log::info!("created PROXY");

    let (tx, _rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = srv::launch_rocket(tx.clone(), log_txs).await;

    // shutdown containers
    remove_container(&docker, &btc_id).await?;
    remove_container(&docker, &lnd_id).await?;
    remove_container(&docker, &proxy_id).await?;

    Ok(())
}
