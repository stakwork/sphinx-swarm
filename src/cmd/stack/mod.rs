mod routes;

use crate::rocket_utils::CmdRequest;
use crate::{dock::*, images, logs};
use anyhow::Result;
use bollard::Docker;
use rocket::tokio::sync::{mpsc, Mutex};
use std::sync::Arc;

pub async fn run(docker: Docker) -> Result<()> {
    let network = "regtest";

    // btc setup
    let btc1 = images::btc("bitcoind", network);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    let (tx, _rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = routes::launch_rocket(tx.clone(), log_txs).await;

    // shutdown containers
    remove_container(&docker, &btc_id).await?;

    Ok(())
}
