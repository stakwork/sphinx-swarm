use anyhow::Result;
use sphinx_swarm::config::{Node, Stack};
use sphinx_swarm::dock::*;
use sphinx_swarm::images::rqbit::RqbitImage;
use sphinx_swarm::images::{dufs::DufsImage, tome::TomeImage, Image};
use sphinx_swarm::rocket_utils::CmdRequest;
use sphinx_swarm::{builder, events, handler, logs, routes};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

// cd /var/lib/docker/volumes/

const DUFS: &str = "dufs";
const RQBIT: &str = "rqbit";
const TOME: &str = "tome";
const JWT_KEY: &str = "asdfasdfasdf";

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

    let proj = "tome";

    println!("=> spawn handler");
    handler::spawn_handler(proj, rx, docker.clone());

    let clients = builder::build_stack(proj, &docker, &stack).await?;
    println!("hydrate clients now!");
    handler::hydrate_clients(clients).await;

    tokio::signal::ctrl_c().await?;

    builder::shutdown_now();

    Ok(())
}

fn make_stack() -> Stack {
    let network = "regtest".to_string();

    let dufs = DufsImage::new(DUFS, "latest", "8080");
    // let bot = BotImage::new("bot", "latest", "3000");
    let rqbit = RqbitImage::new(RQBIT, "latest");
    let tome = TomeImage::new(TOME, "latest", "3000");

    let nodes = vec![Image::Dufs(dufs), Image::Rqbit(rqbit), Image::Tome(tome)];

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
