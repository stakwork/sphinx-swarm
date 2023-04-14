use anyhow::Result;
use rocket::tokio::sync::{mpsc, Mutex};
use sphinx_swarm::config::{Clients, Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::cln::ClnPlugin;
use sphinx_swarm::images::{btc::BtcImage, cln::ClnImage, Image};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::{builder, handler, logs, routes};
use std::sync::Arc;

// docker run -it --privileged --pid=host debian nsenter -t 1 -m -u -n -i sh

// cd /var/lib/docker/volumes/

const BTC: &str = "btc_1";
const CLN1: &str = "cln_1";
const CLN2: &str = "cln_2";
// const LND1: &str = "lnd_1";

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let stack = make_stack();
    let mut clients = builder::build_stack("cln", &docker, &stack).await?;

    let mut skip_setup = false;
    if let Ok(clnskip) = std::env::var("CLN_SKIP_SETUP") {
        if clnskip == "true" {
            skip_setup = true;
        }
    }
    if !skip_setup {
        setup_chans(&mut clients).await?;
    }

    sphinx_swarm::auth::set_jwt_key(&stack.jwt_key);

    handler::hydrate(stack, clients).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    println!("=> spawn handler");
    handler::spawn_handler("cln_test", rx, docker.clone());

    println!("=> launch rocket");
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = routes::launch_rocket(tx.clone(), log_txs).await?;

    Ok(())
}

async fn setup_chans(clients: &mut Clients) -> Result<()> {
    let cln2 = clients.cln.get_mut(CLN2).unwrap();
    let cln2_info = cln2.get_info().await?;
    let cln2_pubkey = hex::encode(cln2_info.id);
    log::info!("CLN2 pubkey {}", &cln2_pubkey);
    make_new_chan(clients, &cln2_pubkey).await?;
    Ok(())
}

async fn make_new_chan(clients: &mut Clients, peer_pubkey: &str) -> Result<()> {
    let cln1 = clients.cln.get_mut(CLN1).unwrap();
    let connected = cln1
        .connect_peer(peer_pubkey, &format!("{}.sphinx", CLN2), "9736")
        .await?;
    let channel = hex::encode(connected.id);
    log::info!("CLN1 connected to CLN2: {}", channel);
    let funded = cln1.try_fund_channel(&channel, 100_000_000).await?;
    log::info!("funded {:?}", hex::encode(funded.tx));
    let addr = cln1.new_addr().await?;

    let btcrpc = clients.bitcoind.get(BTC).unwrap();
    let address = addr.bech32.unwrap();
    btcrpc.test_mine(6, Some(address.clone()))?;
    log::info!("mined 6 blocks to {:?}", address);

    let mut ok = false;
    log::info!("wait for channel to confirm...");
    while !ok {
        let peers = cln1.list_peers().await?;
        for p in peers.peers {
            for c in p.channels {
                // println!("{:?}", c.status);
                if let Some(status) = c.status.get(0) {
                    if status.starts_with("CHANNELD_NORMAL") {
                        log::info!("channel confirmed!!!");
                        ok = true;
                    }
                }
            }
        }
        sleep(1000).await;
    }

    let sent_keysend = cln1.keysend(peer_pubkey, 1_000_000).await?;
    println!(
        "=> sent_keysend to {} {:?}",
        peer_pubkey, sent_keysend.status
    );
    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    // bitcoind
    let v = "v23.0";
    let mut bitcoind = BtcImage::new(BTC, v, &network);
    bitcoind.set_user_password("sphinx", "password");

    let v = "v23.02";
    let mut cln = ClnImage::new(CLN1, v, &network, "9735", "10009");
    let plugins = vec![ClnPlugin::HsmdBroker, ClnPlugin::HtlcInterceptor];
    cln.plugins(plugins);
    cln.links(vec![BTC]);

    let mut cln2 = ClnImage::new(CLN2, v, &network, "9736", "10010");
    cln2.links(vec![BTC]);

    // let v = "v0.15.5-beta";
    // let mut lnd = sphinx_swarm::images::lnd::LndImage::new(LND1, v, &network, "10011", "9737");
    // lnd.http_port = Some("8881".to_string());
    // lnd.links(vec![BTC]);

    let internal_nodes = vec![
        Image::Btc(bitcoind),
        Image::Cln(cln),
        Image::Cln(cln2),
        // Image::Lnd(lnd),
    ];
    let nodes: Vec<Node> = internal_nodes
        .iter()
        .map(|n| Node::Internal(n.to_owned()))
        .collect();
    Stack {
        network,
        nodes,
        host: None,
        users: vec![Default::default()],
        jwt_key: sphinx_swarm::secrets::random_word(16),
    }
}

/*

docker exec -it bitcoind.test sh

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -getinfo

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass createwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass loadwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -generate 6

*/
