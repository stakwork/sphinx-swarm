use crate::routes::CmdRequest;
use crate::{dock::*, images, logs, routes};
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

const N: u8 = 1;
static NODES: Lazy<HashMap<String, u8>> = Lazy::new(|| {
    let mut n = HashMap::new();
    for i in 1..1 + N {
        n.insert(smallhash(&i), i);
    }
    write_nodes_file(&n);
    n
});

pub async fn run(docker: Docker) -> Result<()> {
    let network = "regtest";

    // btc setup
    let btc1 = images::btc("bitcoind", network);
    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");

    // cln setup
    let mut id_map = HashMap::new();
    let mut log_txs = logs::new_log_chans();
    for (tag, i) in NODES.iter() {
        let name = format!("cln{}", i);
        let index = (*i as u16) * 3u16;
        let cln1 = images::cln_vls(&name, index, vec!["bitcoind"], network);
        let id = create_and_start(&docker, cln1).await?;
        id_map.insert(tag, id);
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
    log::info!("🚀");
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = routes::launch_rocket(tx.clone(), log_txs).await;

    // shutdown containers
    remove_container(&docker_arc, &btc_id).await?;
    for (_tag, id) in id_map.iter() {
        remove_container(&docker_arc, &id).await?;
    }
    Ok(())
}

const NODES_FILE_PATH: &str = "app/public/nodes.json";
fn write_nodes_file(n: &HashMap<String, u8>) {
    let st = serde_json::to_string_pretty(n).expect("failed to make json string");
    let mut file = File::create(NODES_FILE_PATH).expect("create failed");
    file.write_all(st.as_bytes()).expect("write failed");
}

// first 4 bytes of hash
fn smallhash<T: Hash>(t: &T) -> String {
    do_hash(&t)[..4].to_vec().to_base58()
}
fn do_hash<T: Hash>(t: &T) -> [u8; 8] {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    let r: u64 = s.finish();
    r.to_be_bytes()
}
