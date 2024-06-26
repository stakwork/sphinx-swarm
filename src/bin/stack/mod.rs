use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::backup::backup_and_delete_volumes_cron;
use sphinx_swarm::builder;
use sphinx_swarm::config::{load_config_file, put_config_file, Stack};
use sphinx_swarm::handler;
use sphinx_swarm::routes;
use sphinx_swarm::{dock::*, events, logs, rocket_utils::CmdRequest};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[rocket::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let proj = "stack";
    let stack: Stack = load_config_file(proj).await.expect("YAML CONFIG FAIL");

    // put the jwt key into a var
    sphinx_swarm::auth::set_jwt_key(&stack.jwt_key);
    // hyrate the "stack" without clients
    handler::hydrate_stack(stack.clone()).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();
    let log_txs = Arc::new(Mutex::new(log_txs));

    let event_txs = events::new_event_chan();

    println!("=> launch rocket");
    tokio::spawn(async move {
        // launch rocket
        let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
        log::info!("🚀 => http://localhost:{}", port);
        let _r = routes::launch_rocket(tx.clone(), log_txs, event_txs)
            .await
            .unwrap();
        // ctrl-c shuts down rocket
        builder::shutdown_now();
    });

    handler::spawn_handler(proj, rx, docker.clone());

    let clients = builder::build_stack(proj, &docker, &stack).await?;
    put_config_file(proj, &stack).await;

    println!("hydrate clients now!");
    handler::hydrate_clients(clients).await;

    if let Some(nn) = stack.auto_update {
        let cron_handler_res = builder::auto_updater(proj, docker, nn).await;
        if let Err(e) = cron_handler_res {
            log::error!("CRON failed {:?}", e);
        }
    }

    backup_and_delete_volumes_cron().await?;

    tokio::signal::ctrl_c().await?;

    builder::shutdown_now();

    Ok(())
}
