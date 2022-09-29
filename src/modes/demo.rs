use crate::api::*;
use crate::images;
use crate::routes;
use anyhow::Result;
use base58::ToBase58;
use bollard::Docker;
use once_cell::sync::Lazy;
use rocket::tokio::sync::mpsc;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::io::Write;

static NODES: Lazy<HashMap<String, u8>> = Lazy::new(|| {
    let mut n = HashMap::new();
    for i in 1..33 {
        // first 4 bytes of hash
        let h = do_hash(&i)[..4].to_vec();
        n.insert(h.to_base58(), i);
    }
    let st = serde_json::to_string_pretty(&n).expect("failed to make json string");
    let mut file = std::fs::File::create("nodes.json").expect("create failed");
    file.write_all(st.as_bytes()).expect("write failed");
    n
});

pub async fn run(docker: &Docker) -> Result<()> {
    let btc1 = images::btc("bitcoind");
    let _id = create_and_start(docker, btc1).await?;
    log::info!("created bitcoind");
    for i in 1..NODES.len() as u16 {
        let name = format!("cln{}", i);
        let cln1 = images::cln_vls(name.as_str(), i, vec!["bitcoind"]);
        create_and_start(docker, cln1).await?;
        log::info!("created {}", name);
    }
    let (tx, _rx) = mpsc::channel(1000);
    log::info!("🚀");
    let _r = routes::launch_rocket(tx.clone()).await;
    Ok(())
}

fn do_hash<T: Hash>(t: &T) -> [u8; 8] {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    let r: u64 = s.finish();
    r.to_be_bytes()
}
