use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::config::{Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::cln::ClnPlugin;
use sphinx_swarm::images::lss::LssImage;
use sphinx_swarm::images::{btc::BtcImage, cln::ClnImage, lnd::LndImage, proxy::ProxyImage, Image};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::setup::*;
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
        setup_cln_chans(&mut clients, &stack.nodes, CLN1, CLN2, BTC).await?;
        if do_test_proxy() {
            setup_lnd_chans(&mut clients, &stack.nodes, CLN1, LND_1, BTC).await?;
        }
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

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    let mut internal_nodes = Vec::new();

    // let cln_plugins = vec![ClnPlugin::HsmdBroker, ClnPlugin::HtlcInterceptor];
    // let cln_plugins = vec![ClnPlugin::HsmdBroker];
    // let cln_plugins = vec![ClnPlugin::HtlcInterceptor];
    let cln_plugins = vec![];

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
        custom_2b_domain: None,
        global_mem_limit: None,
        backup_services: None,
        backup_files: None,
        lightning_peers: None,
        auto_restart: None,
        ssl_cert_last_modified: None,
        instance_id: None,
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
