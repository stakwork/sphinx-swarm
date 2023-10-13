use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::cmd::PayKeysend;
use sphinx_swarm::config::{Clients, Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::cln::ClnPlugin;
use sphinx_swarm::images::lss::LssImage;
use sphinx_swarm::images::{btc::BtcImage, cln::ClnImage, lnd::LndImage, proxy::ProxyImage, Image};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::utils::domain;
use sphinx_swarm::{builder, events, handler, logs, routes};
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

const LND_TLV_LEN: usize = 1944;

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let stack = make_stack();
    log::info!("STACK {:?}", stack);

    sphinx_swarm::auth::set_jwt_key(&stack.jwt_key);
    handler::hydrate_stack(stack.clone()).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    println!("=> launch rocket");
    let log_txs = Arc::new(Mutex::new(log_txs));

    let event_tx = events::new_event_chan();

    tokio::spawn(async move {
        let _r = routes::launch_rocket(tx.clone(), log_txs, event_tx)
            .await
            .unwrap();
        // ctrl-c shuts down rocket
        builder::shutdown_now();
    });

    let proj = "cln_test";

    println!("=> spawn handler");
    handler::spawn_handler(proj, rx, docker.clone());

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

    if let Some(nn) = stack.auto_update {
        let _cron_handler = builder::auto_updater(proj, docker, nn).await?;
    }

    tokio::signal::ctrl_c().await?;

    builder::shutdown_now();

    Ok(())
}

async fn get_pubkey_cln(clients: &mut Clients, node_id: &str) -> Result<String> {
    let client = clients.cln.get_mut(node_id).unwrap();
    let info = client.get_info().await?;
    let pubkey = hex::encode(info.id);
    Ok(pubkey)
}

async fn get_pubkey_lnd(clients: &mut Clients, node_id: &str) -> Result<String> {
    let lnd1 = clients.lnd.get_mut(node_id).unwrap();
    let lnd1_info = lnd1.get_info().await?;
    Ok(lnd1_info.identity_pubkey)
}

async fn setup_cln_chans(clients: &mut Clients, nodes: &Vec<Node>) -> Result<()> {
    let cln2_pubkey = get_pubkey_cln(clients, CLN2).await?;
    if let Some(node) = nodes.iter().find(|n| n.name() == CLN2) {
        log::info!("CLN2 pubkey {}", &cln2_pubkey);
        let n = node.as_internal()?.as_cln()?;
        new_chan_from_cln1(clients, CLN2, &cln2_pubkey, &n.peer_port).await?;
        // keysend send
        cln_keysend_to(clients, CLN1, &cln2_pubkey, 1_000_000, false).await?;
        sleep(1000).await;
        cln_keysend_to(clients, CLN1, &cln2_pubkey, 1_000_000, false).await?;
        sleep(1000).await;
        // keysend receive
        let cln1_pubkey = get_pubkey_cln(clients, CLN1).await?;
        cln_keysend_to(clients, CLN2, &cln1_pubkey, 500_000, true).await?;
    } else {
        log::error!("CLN2 not found!");
    }
    Ok(())
}

async fn setup_lnd_chans(clients: &mut Clients, nodes: &Vec<Node>) -> Result<()> {
    if !do_test_proxy() {
        return Ok(());
    }
    let lnd1_pubkey = get_pubkey_lnd(clients, LND_1).await?;
    if let Some(node) = nodes.iter().find(|n| n.name() == LND_1) {
        log::info!("LND1 pubkey {}", &lnd1_pubkey);
        let n = node.as_internal()?.as_lnd()?;
        new_chan_from_cln1(clients, LND_1, &lnd1_pubkey, &n.peer_port).await?;
        // keysend send
        cln_keysend_to(clients, CLN1, &lnd1_pubkey, 1_000_000, false).await?;
        sleep(1000).await;
        // keysend send
        cln_keysend_to(clients, CLN1, &lnd1_pubkey, 1_000_000, false).await?;
        sleep(59000).await;
        // keysend receive
        let cln1_pubkey = get_pubkey_cln(clients, CLN1).await?;
        log::info!("lnd send 1");
        lnd_keysend_to(clients, LND_1, &cln1_pubkey, 500_000, false).await?;
        log::info!("lnd send 2");
        lnd_keysend_to(clients, LND_1, &cln1_pubkey, 500_000, true).await?;
    }
    Ok(())
}

async fn new_chan_from_cln1(
    clients: &mut Clients,
    peer_name: &str,
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
        .connect_peer(peer_pubkey, &domain(peer_name), peer_port)
        .await?;
    let channel = hex::encode(connected.id);
    log::info!("CLN1 connected to {}: {}", peer_name, channel);
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

    Ok(())
}

async fn cln_keysend_to(
    clients: &mut Clients,
    sender_id: &str,
    recip_pubkey: &str,
    amt: u64,
    do_tlv: bool,
) -> Result<()> {
    let tlv_opt = if do_tlv {
        let mut tlvs = std::collections::HashMap::new();
        tlvs.insert(133773310, [9u8; 1124].to_vec()); // (1207 ok, 1208 not) 603 bytes max
        Some(tlvs)
    } else {
        None
    };

    let cln1 = clients.cln.get_mut(sender_id).unwrap();
    match cln1
        .keysend(recip_pubkey, amt, None, None, None, tlv_opt)
        .await
    {
        Ok(sent_keysend) => println!(
            "[CLN] => sent_keysend to {} {:?}",
            recip_pubkey, sent_keysend.status
        ),
        Err(e) => {
            println!("[CLN] keysend err {:?}", e)
        }
    };
    Ok(())
}

async fn lnd_keysend_to(
    clients: &mut Clients,
    sender_id: &str,
    recip_pubkey: &str,
    amt: u64,
    do_tlv: bool,
) -> Result<()> {
    let tlv_opt = if do_tlv {
        let mut tlvs = std::collections::HashMap::new();
        tlvs.insert(133773310, [9u8; LND_TLV_LEN].to_vec()); // (1124 ok, 1224 not)
        Some(tlvs)
    } else {
        None
    };

    let pk = PayKeysend {
        dest: recip_pubkey.to_string(),
        amt: (amt / 1000) as i64,
        tlvs: tlv_opt,
        ..Default::default()
    };
    log::info!("pk {:?}", &pk);
    let lnd1 = clients.lnd.get_mut(sender_id).unwrap();
    match lnd1.pay_keysend(pk).await {
        Ok(sent_keysend) => println!(
            "[LND] => sent_keysend to {} {:?}",
            recip_pubkey, sent_keysend
        ),
        Err(e) => {
            println!("[LND] keysend err {:?}", e)
        }
    };
    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    let mut internal_nodes = Vec::new();

    // let cln_plugins = vec![ClnPlugin::HsmdBroker, ClnPlugin::HtlcInterceptor];
    // let cln_plugins = vec![ClnPlugin::HsmdBroker];
    let cln_plugins = vec![ClnPlugin::HtlcInterceptor];

    // bitcoind
    let v = "v23.0";
    let mut bitcoind = BtcImage::new(BTC, v, &network);
    bitcoind.set_user_password("sphinx", "password");
    internal_nodes.push(Image::Btc(bitcoind));

    // LSS
    let lss = LssImage::new(LSS, "0.0.4", "55551");
    if cln_plugins.contains(&ClnPlugin::HsmdBroker) {
        internal_nodes.push(Image::Lss(lss));
    }

    // CLN1
    let v = "latest";
    let mut cln = ClnImage::new(CLN1, v, &network, "9735", "10009");
    cln.plugins(cln_plugins.clone());
    cln.links(vec![BTC, LSS]);
    internal_nodes.push(Image::Cln(cln));

    // CLN2
    let mut cln2 = ClnImage::new(CLN2, v, &network, "9736", "10010");
    cln2.links(vec![BTC]);
    internal_nodes.push(Image::Cln(cln2));

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
        ip: None,
        // test cln2 updating
        auto_update: Some(vec![CLN2.to_string()]),
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
