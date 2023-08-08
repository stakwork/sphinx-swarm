use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::config::{Clients, Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::cln::ClnPlugin;
use sphinx_swarm::images::lss::LssImage;
use sphinx_swarm::images::{btc::BtcImage, cln::ClnImage, lnd::LndImage, proxy::ProxyImage, Image};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::utils::domain;
use sphinx_swarm::{builder, handler, logs, routes};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

// docker run -it --privileged --pid=host debian nsenter -t 1 -m -u -n -i sh

// cd /var/lib/docker/volumes/

const BTC: &str = "btc_1";
const CLN1: &str = "cln_1";
const CLN2: &str = "cln_2";
const LSS: &str = "lss_1";
const JWT_KEY: &str = "e8int45s0pofgtye";
const LND_1: &str = "lnd_1";

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let stack = make_stack();

    sphinx_swarm::auth::set_jwt_key(&stack.jwt_key);
    handler::hydrate_stack(stack.clone()).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    println!("=> launch rocket");
    let log_txs = Arc::new(Mutex::new(log_txs));
    tokio::spawn(async move {
        let _r = routes::launch_rocket(tx.clone(), log_txs).await.unwrap();
        // ctrl-c shuts down rocket
        builder::shutdown_now();
    });

    println!("=> spawn handler");
    handler::spawn_handler("cln_test", rx, docker.clone());

    let mut clients = builder::build_stack("cln", &docker, &stack).await?;

    let mut skip_setup = false;
    if let Ok(clnskip) = std::env::var("CLN_SKIP_SETUP") {
        if clnskip == "true" {
            skip_setup = true;
        }
    }
    if !skip_setup {
        setup_cln_chans(&mut clients, &stack.nodes).await?;
        setup_lnd_chans(&mut clients, &stack.nodes).await?;
    }

    println!("hydrate clients now!");
    handler::hydrate_clients(clients).await;

    tokio::signal::ctrl_c().await?;

    builder::shutdown_now();

    Ok(())
}

async fn setup_cln_chans(clients: &mut Clients, nodes: &Vec<Node>) -> Result<()> {
    let cln2 = clients.cln.get_mut(CLN2).unwrap();
    let cln2_info = cln2.get_info().await?;
    let cln2_pubkey = hex::encode(cln2_info.id);
    if let Some(node) = nodes.iter().find(|n| n.clone().name() == CLN2) {
        log::info!("CLN2 pubkey {}", &cln2_pubkey);
        let n = node.as_internal()?.as_cln()?;
        make_new_chan(clients, CLN2, &cln2_pubkey, &n.peer_port).await?;
    } else {
        log::error!("CLN2 not found!");
    }
    Ok(())
}

async fn setup_lnd_chans(clients: &mut Clients, nodes: &Vec<Node>) -> Result<()> {
    if !do_test_proxy() {
        return Ok(());
    }
    let lnd1 = clients.lnd.get_mut(LND_1).unwrap();
    let lnd1_info = lnd1.get_info().await?;
    let lnd1_pubkey = lnd1_info.identity_pubkey;
    if let Some(node) = nodes.iter().find(|n| n.clone().name() == LND_1) {
        log::info!("LND1 pubkey {}", &lnd1_pubkey);
        let n = node.as_internal()?.as_lnd()?;
        make_new_chan(clients, LND_1, &lnd1_pubkey, &n.peer_port).await?;
    }
    Ok(())
}

async fn make_new_chan(
    clients: &mut Clients,
    node_name: &str,
    peer_pubkey: &str,
    peer_port: &str,
) -> Result<()> {
    let cln1 = clients.cln.get_mut(CLN1).unwrap();

    // skip if already have a chan
    let peers = cln1.list_peers().await?;
    for p in peers
        .peers
        .iter()
        .filter(|peer| hex::encode(peer.id.clone()) == peer_pubkey)
    {
        if p.channels.len() > 0 {
            log::info!("skipping new channel setup");
            return Ok(());
        }
    }

    let connected = cln1
        .connect_peer(peer_pubkey, &domain(node_name), peer_port)
        .await?;
    let channel = hex::encode(connected.id);
    log::info!("CLN1 connected to {}: {}", node_name, channel);
    let funded = cln1.try_fund_channel(&channel, 100_000_000, None).await?;
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
        for p in peers
            .peers
            .into_iter()
            .filter(|peer| hex::encode(peer.id.clone()) == peer_pubkey)
        {
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

    match cln1
        .keysend(peer_pubkey, 1_000_000, None, None, None, None)
        .await
    {
        Ok(sent_keysend) => println!(
            "=> sent_keysend to {} {:?}",
            peer_pubkey, sent_keysend.status
        ),
        Err(e) => {
            println!("keysend err {:?}", e)
        }
    };

    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    // bitcoind
    let v = "v23.0";
    let mut bitcoind = BtcImage::new(BTC, v, &network);
    bitcoind.set_user_password("sphinx", "password");

    let lss = LssImage::new(LSS, "0.0.4", "55551");

    let v = "v23.02";
    let mut cln = ClnImage::new(CLN1, v, &network, "9735", "10009");
    // let plugins = vec![ClnPlugin::HsmdBroker, ClnPlugin::HtlcInterceptor];
    // let plugins = vec![ClnPlugin::HsmdBroker];
    let cln_plugins = vec![ClnPlugin::HtlcInterceptor];
    cln.plugins(cln_plugins.clone());
    cln.links(vec![BTC, LSS]);

    let mut cln2 = ClnImage::new(CLN2, v, &network, "9736", "10010");
    cln2.links(vec![BTC]);

    let mut internal_nodes = vec![Image::Btc(bitcoind), Image::Cln(cln), Image::Cln(cln2)];
    if cln_plugins.contains(&ClnPlugin::HsmdBroker) {
        internal_nodes.push(Image::Lss(lss));
    }

    if do_test_proxy() {
        let v = "v0.16.2-beta";
        let mut lnd = LndImage::new(LND_1, v, &network, "10011", "9737");
        lnd.http_port = Some("8881".to_string());
        lnd.links(vec![BTC]);

        let v = "0.1.44";
        let mut proxy = ProxyImage::new("proxy", v, &network, "11111", "5555");
        println!("=====================");
        println!("proxy admin token {:?}", proxy.admin_token);
        println!("=====================");
        proxy.new_nodes(Some("0".to_string()));
        proxy.links(vec![LND_1]);

        let proxy_test_nodes = vec![Image::Lnd(lnd), Image::Proxy(proxy)];
        internal_nodes.extend_from_slice(&proxy_test_nodes);
    }

    let nodes: Vec<Node> = internal_nodes
        .iter()
        .map(|n| Node::Internal(n.to_owned()))
        .collect();
    Stack {
        network,
        nodes,
        host: None,
        users: vec![Default::default()],
        jwt_key: JWT_KEY.to_string(),
        ready: false,
    }
}

fn do_test_proxy() -> bool {
    if let Ok(test_proxy) = std::env::var("TEST_PROXY") {
        if test_proxy == String::from("true") {
            return true;
        }
    }
    false
}

/*

docker exec -it bitcoind.test sh

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -getinfo

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass createwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass loadwallet wallet

bitcoin-cli -regtest -rpcuser=evan -rpcpassword=thepass -generate 6

*/
