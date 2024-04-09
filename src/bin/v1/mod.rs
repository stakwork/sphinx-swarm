use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::config::{Clients, Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::cln::ClnPlugin;
use sphinx_swarm::images::{
    broker::BrokerImage, btc::BtcImage, cln::ClnImage, mixer::MixerImage, tribes::TribesImage,
    Image,
};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::setup::{get_pubkey_cln, setup_cln_chans};
use sphinx_swarm::utils::domain;
use sphinx_swarm::{builder, events, handler, logs, routes};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

// cd /var/lib/docker/volumes/

// docker cp sphinx.yml cln_1.sphinx:/root/.lightning/regtest/sphinx.yml

// docker exec -it cln_1.sphinx sh

const BTC: &str = "btc_1";
const CLN1: &str = "cln_1";
const CLN2: &str = "cln_2";
const CLN3: &str = "cln_3";
const BROKER1: &str = "broker_1";
const BROKER2: &str = "broker_2";
const BROKER3: &str = "broker_3";
const MIXER1: &str = "mixer_1";
const MIXER2: &str = "mixer_2";
const MIXER3: &str = "mixer_3";
const TRIBES1: &str = "tribes_1";
// const TRIBES2: &str = "tribes_2";
const TRIBES3: &str = "tribes_3";
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
        setup_cln_chans(&mut clients, &stack.nodes, CLN3, CLN2, BTC).await?;
    }

    try_check_2_hops(&mut clients, CLN1, CLN3).await;

    println!("hydrate clients now!");
    handler::hydrate_clients(clients).await;

    tokio::signal::ctrl_c().await?;

    builder::shutdown_now();

    Ok(())
}

async fn try_check_2_hops(clients: &mut Clients, node1: &str, node3: &str) {
    for i in 0..200 {
        let res = check_2_hops(clients, node1, node3).await;
        if res.is_ok() {
            return;
        }
        log::info!("retrying get_route to CLN3: {}...", i);
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

async fn check_2_hops(clients: &mut Clients, node1: &str, node3: &str) -> Result<()> {
    let cln3_pubkey = get_pubkey_cln(clients, node3).await?;
    let cln1 = clients.cln.get_mut(node1).unwrap();
    let res = cln1.get_route(&cln3_pubkey, 1000).await?;
    if res.route.len() < 2 {
        return Err(anyhow::anyhow!("no route found"));
    }
    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    let cln_plugins = vec![ClnPlugin::HtlcInterceptor];

    // bitcoind
    let v = "v23.0";
    let mut bitcoind = BtcImage::new(BTC, v, &network);
    bitcoind.set_user_password("sphinx", "password");

    // CLN1
    let seed1 = [43; 32];
    let v = "latest";
    let mut cln1 = ClnImage::new(CLN1, v, &network, "9735", "10009");
    cln1.set_seed(seed1);
    cln1.plugins(cln_plugins.clone());
    cln1.links(vec![BTC]);

    let mut broker1 = BrokerImage::new(BROKER1, v, &network, "1883", None);
    broker1.set_seed(&hex::encode(&seed1));
    broker1.set_logs("login,pubsub");
    let broker1ip = format!("{}:{}", domain(&broker1.name), &broker1.mqtt_port);

    let mut mixer1 = MixerImage::new(MIXER1, v, &network, "8080");
    mixer1.links(vec![CLN1, BROKER1, BROKER2]);
    mixer1.set_default_tribe_pubkey(
        "0374b91a4c726f0c097a2643f63983491d5afabdc9bdd8576096e2b5580107bf03",
    );
    mixer1.set_log_level("debug");
    let mixer1pk = "03e6fe3af927476bcb80f2bc52bc0012c5ea92cc03f9165a4af83dbb214e296d08";

    let mut tribes1 = TribesImage::new(TRIBES1, v, &network, "8801");
    tribes1.links(vec![BROKER1]);

    // CLN2
    let seed2 = [44; 32];
    let mut cln2 = ClnImage::new(CLN2, v, &network, "9736", "10010");
    cln2.set_seed(seed2);
    // NO HTLC INTERCEPTOR FOR ROUTING NODE
    // cln2.plugins(cln_plugins.clone());
    cln2.links(vec![BTC]);

    let mut broker2 = BrokerImage::new(BROKER2, v, &network, "1884", None);
    broker2.set_seed(&hex::encode(&seed2));
    broker2.set_logs("login,pubsub");
    let broker2ip = format!("{}:{}", domain(&broker2.name), &broker2.mqtt_port);

    let mut mixer2 = MixerImage::new(MIXER2, v, &network, "8081");
    mixer2.links(vec![CLN2, BROKER2]);
    mixer2.set_log_level("debug");
    // NO GRPC WITH GATEWAY NEEDED FOR ROUTING NODE
    mixer2.set_no_gateway();
    let mixer2pk = "036bebdc8ad27b5d9bd14163e9fea5617ac8618838aa7c0cae19d43391a9feb9db";

    // CLN3
    let seed3 = [45; 32];
    let mut cln3 = ClnImage::new(CLN3, v, &network, "9737", "10011");
    cln3.set_seed(seed3);
    cln3.plugins(cln_plugins.clone());
    cln3.links(vec![BTC]);

    let mut broker3 = BrokerImage::new(BROKER3, v, &network, "1885", None);
    broker3.set_seed(&hex::encode(&seed3));
    broker3.set_logs("login,pubsub");
    let broker3ip = format!("{}:{}", domain(&broker3.name), &broker3.mqtt_port);

    let mut mixer3 = MixerImage::new(MIXER3, v, &network, "8082");
    mixer3.links(vec![CLN3, BROKER3, BROKER2]);
    mixer3.set_default_tribe_pubkey(
        "038d3c5f8392dd91e7a7289e92ee6cb6ded5db6dbfd06c8c6fb8b42fe511672627",
    );
    mixer3.set_log_level("debug");
    let mixer3pk = "030f5205642b40c64ac5c575f4f365ca90b692f13808b46d827fdb1b6026a3e6c2";

    let mut tribes3 = TribesImage::new(TRIBES3, v, &network, "8803");
    tribes3.links(vec![BROKER3]);

    // 2 -> 1 and 3
    mixer2.set_initial_peers(&format!(
        "{}@{},{}@{}",
        mixer1pk, broker1ip, mixer3pk, broker3ip
    ));

    // 1 and 3 -> 2
    mixer3.set_initial_peers(&format!("{}@{}", mixer2pk, broker2ip));
    mixer1.set_initial_peers(&format!("{}@{}", mixer2pk, broker2ip));

    let nodes = vec![
        // bitcoin
        Image::Btc(bitcoind),
        // 1
        Image::Cln(cln1),
        Image::Broker(broker1),
        Image::Mixer(mixer1),
        Image::Tribes(tribes1),
        // 2 (routing)
        Image::Cln(cln2),
        Image::Broker(broker2),
        Image::Mixer(mixer2),
        // 3
        Image::Cln(cln3),
        Image::Broker(broker3),
        Image::Mixer(mixer3),
        Image::Tribes(tribes3),
    ];

    let ns: Vec<Node> = nodes.iter().map(|n| Node::Internal(n.to_owned())).collect();
    Stack {
        network,
        nodes: ns,
        host: None,
        users: vec![Default::default()],
        jwt_key: JWT_KEY.to_string(),
        ready: false,
        ..Default::default()
    }
}
