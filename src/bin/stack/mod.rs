use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::builder;
use sphinx_swarm::config::{load_config_file, put_config_file, Stack};
use sphinx_swarm::handler;
use sphinx_swarm::routes;
use sphinx_swarm::{dock::*, logs, rocket_utils::CmdRequest};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[rocket::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let proj = "stack";
    let stack: Stack = load_config_file(proj).await;
    let clients = builder::build_stack(proj, &docker, &stack).await?;
    put_config_file(proj, &stack).await;
    // put the jwt key into a var
    sphinx_swarm::auth::set_jwt_key(&stack.jwt_key);

    handler::hydrate(stack, clients).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    handler::spawn_handler(proj, rx, docker.clone());

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = routes::launch_rocket(tx.clone(), log_txs).await?;

    // for (_, id) in ids {
    //     stop_and_remove(&docker, &id).await?;
    // }

    Ok(())
}
