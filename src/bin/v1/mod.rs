use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::config::{Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::cln::ClnPlugin;
use sphinx_swarm::images::{
    broker::BrokerImage, btc::BtcImage, cln::ClnImage, mixer::MixerImage, Image,
};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::setup::setup_cln_chans;
use sphinx_swarm::{builder, events, handler, logs, routes};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

// cd /var/lib/docker/volumes/

// docker cp sphinx.yml cln_1.sphinx:/root/.lightning/regtest/sphinx.yml

// docker exec -it cln_1.sphinx sh

const BTC: &str = "btc_1";
const CLN1: &str = "cln_1";
const CLN2: &str = "cln_2";
const BROKER1: &str = "broker_1";
const BROKER2: &str = "broker_2";
const MIXER1: &str = "mixer_1";
const MIXER2: &str = "mixer_2";
const JWT_KEY: &str = "f8int45s0pofgtye";

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

    let proj = "sphinx";

    println!("=> spawn handler");
    handler::spawn_handler(proj, rx, docker.clone());

    let mut clients = builder::build_stack(proj, &docker, &stack).await?;

    let mut skip_setup = false;
    if let Ok(clnskip) = std::env::var("CLN_SKIP_SETUP") {
        if clnskip == "true" {
            skip_setup = true;
        }
    }
    if !skip_setup {
        setup_cln_chans(&mut clients, &stack.nodes, CLN1, CLN2, BTC).await?;
    }

    println!("hydrate clients now!");
    handler::hydrate_clients(clients).await;

    tokio::signal::ctrl_c().await?;

    builder::shutdown_now();

    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    let mut nodes = Vec::new();

    let cln_plugins = vec![ClnPlugin::HtlcInterceptor];

    // bitcoind
    let v = "v23.0";
    let mut bitcoind = BtcImage::new(BTC, v, &network);
    bitcoind.set_user_password("sphinx", "password");
    nodes.push(Image::Btc(bitcoind));

    // CLN1
    let seed1 = [43; 32];
    let v = "latest";
    let mut cln = ClnImage::new(CLN1, v, &network, "9735", "10009");
    cln.set_seed(seed1);
    cln.plugins(cln_plugins.clone());
    cln.links(vec![BTC]);
    nodes.push(Image::Cln(cln));

    let mut broker = BrokerImage::new(BROKER1, v, &network, "1883", None);
    broker.set_seed(&hex::encode(&seed1));
    broker.set_logs("login,pubsub");
    nodes.push(Image::Broker(broker));

    let mut mixer = MixerImage::new(MIXER1, v, &network, "8080");
    mixer.links(vec![CLN1, BROKER1]);
    nodes.push(Image::Mixer(mixer));

    // CLN2
    let seed2 = [44; 32];
    let mut cln2 = ClnImage::new(CLN2, v, &network, "9736", "10010");
    cln2.set_seed(seed2);
    cln2.plugins(cln_plugins.clone());
    cln2.links(vec![BTC]);
    nodes.push(Image::Cln(cln2));

    let mut broker2 = BrokerImage::new(BROKER2, v, &network, "1884", None);
    broker2.set_seed(&hex::encode(&seed2));
    broker2.set_logs("login,pubsub");
    nodes.push(Image::Broker(broker2));

    let mut mixer2 = MixerImage::new(MIXER2, v, &network, "8081");
    mixer2.links(vec![CLN2, BROKER2]);
    nodes.push(Image::Mixer(mixer2));

    let ns: Vec<Node> = nodes.iter().map(|n| Node::Internal(n.to_owned())).collect();
    Stack {
        network,
        nodes: ns,
        host: None,
        users: vec![Default::default()],
        jwt_key: JWT_KEY.to_string(),
        ready: false,
        ip: None,
        auto_update: None,
    }
}
