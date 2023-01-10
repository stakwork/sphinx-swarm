mod srv;

use crate::conn::bitcoin::bitcoinrpc::BitcoinRPC;
use crate::images::BtcImage;
use crate::rocket_utils::*;
use crate::{dock::*, env, images, logs};
use anyhow::Result;
use base58::ToBase58;
use bollard::Docker;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use rocket::tokio;
use rocket::tokio::sync::{broadcast, mpsc, Mutex};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::Arc;

const N: u8 = 2;
static NODES: Lazy<HashMap<String, u8>> = Lazy::new(|| {
    let mut n = HashMap::new();
    for i in 1..1 + N {
        let idx = i * 3;
        n.insert(smallhash(&idx), idx);
    }
    write_nodes_file(&n);
    n
});

pub async fn run(docker: Docker) -> Result<()> {
    let proj = "demo";
    let network = "regtest";

    // btc setup
    let btc_node = BtcImage::new("bitcoind", "23.0", network, "foo");

    // Get Bitcoin Info
    let btc1 = images::btc(proj, &btc_node);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    let bitcoin = BitcoinRPC::new(&btc_node, "http://127.0.0.1", "18443")?;
    let info = bitcoin.get_info()?;
    log::info!("bitcoind info {:?}", info);

    // cln setup
    let mut id_map = HashMap::new();
    let mut log_txs = logs::new_log_chans();
    for (tag, i) in NODES.iter() {
        let name = format!("cln{}", i);
        let cln1 = images::cln_vls(proj, &name, network, *i as u16, &btc_node);
        let id = create_and_start(&docker, cln1).await?;
        id_map.insert(tag, id);
        // add in default env var $CLN
        env::add_to_env(tag, "CLN", &format!("lightning-cli --network={}", network)).await;
        env::add_to_env(tag, "HOST", "host.docker.internal").await;
        // streaming logs
        let mut stream = logs_stream(&docker, &name);
        let (log_tx, _) = broadcast::channel(1000);
        logs::collect_logs(&tag, log_tx.clone());
        log_txs.insert(tag.clone(), log_tx.clone());
        tokio::spawn(async move {
            while let Some(lg) = stream.next().await {
                if let Some(msg) = match_stream(lg) {
                    let _ = log_tx.send(String::from_utf8_lossy(&msg).to_string());
                }
            }
        });
        log::info!("created {}", name);
    }

    // commands for "docker exec"
    let (tx, mut rx) = mpsc::channel::<CmdRequest>(1000);
    let docker_arc = Arc::new(docker);
    let docker_ = docker_arc.clone();
    let id_map_ = id_map.clone();
    tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            if let Some(node_id) = id_map_.get(&cmd.tag) {
                if let Ok(ress) = exec(&docker_, &node_id, &cmd.message).await {
                    let _ = cmd.reply_tx.send(ress);
                }
            }
        }
    });

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = srv::launch_rocket(tx.clone(), log_txs).await;

    // shutdown containers
    remove_container(&docker_arc, &btc_id).await?;
    for (_tag, id) in id_map.iter() {
        remove_container(&docker_arc, &id).await?;
    }
    Ok(())
}

const NODES_FILE_PATH: &str = "src/modes/demo/app/public/nodes.json";
fn write_nodes_file(n: &HashMap<String, u8>) {
    let st = serde_json::to_string_pretty(n).expect("failed to make json string");
    let mut file = File::create(NODES_FILE_PATH).expect("create failed");
    file.write_all(st.as_bytes()).expect("write failed");
}

// first 2 bytes of hash
fn smallhash<T: Hash>(t: &T) -> String {
    do_hash(&t)[..2].to_vec().to_base58()
}
fn do_hash<T: Hash>(t: &T) -> [u8; 8] {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    let r: u64 = s.finish();
    r.to_be_bytes()
}
