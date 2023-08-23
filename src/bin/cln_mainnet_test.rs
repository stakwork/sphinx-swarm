use anyhow::Result;
use rocket::tokio::sync::{mpsc, Mutex};
use sphinx_swarm::config::{ExternalNode, ExternalNodeType, Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::{cln::ClnImage, Image};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::{builder, handler, logs, routes};
use std::sync::Arc;

const BTC: &str = "btc_1";
const CLN1: &str = "cln_1";

#[rocket::main]
pub async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let proj = "cln_mainnet_test";
    let stack = make_stack();
    let clients = builder::build_stack(proj, &docker, &stack).await?;

    sphinx_swarm::auth::set_jwt_key(&stack.jwt_key);

    handler::hydrate(stack, clients).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    println!("=> spawn handler");
    handler::spawn_handler(proj, rx, docker.clone());

    println!("=> launch rocket");
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = routes::launch_rocket(tx.clone(), log_txs).await?;

    Ok(())
}

fn make_stack() -> Stack {
    let mbtc = std::env::var("CLN_MAINNET_TEST_BTC").expect("no CLN_MAINNET_TEST_BTC found ");

    let parsed = url::Url::parse(&mbtc).expect("couldnt parse btc url");
    println!("scheme {:?}", parsed.scheme());
    println!("user {:?}", parsed.username());
    println!("pass {:?}", parsed.password());
    println!("host {:?}", parsed.host());

    let network = "bitcoin".to_string();

    let btc = Node::External(ExternalNode::new(BTC, ExternalNodeType::Btc, &mbtc));

    let mut cln1 = ClnImage::new(CLN1, "0.1.0", &network, "9735", "10009");
    cln1.links(vec![BTC]);

    let nodes = vec![btc, Node::Internal(Image::Cln(cln1))];

    Stack {
        network,
        nodes,
        host: None,
        users: vec![Default::default()],
        jwt_key: sphinx_swarm::secrets::random_word(16),
        ready: false,
        ip: None,
    }
}
