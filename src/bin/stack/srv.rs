use fs::{relative, FileServer};
use rocket::*;
use sphinx_swarm::logs::LogChans;
use sphinx_swarm::rocket_utils::{CmdRequest, Result, CORS};
use sphinx_swarm::routes::{all_options, cmd, login, logs, logstream, refresh_jwt};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub async fn launch_rocket(
    tx: mpsc::Sender<CmdRequest>,
    log_txs: Arc<Mutex<LogChans>>,
) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/", FileServer::from(relative!("src/bin/stack/app/dist")))
        .mount(
            "/api/",
            routes![cmd, logs, logstream, login, refresh_jwt, all_options],
        )
        .attach(CORS)
        .manage(tx)
        .manage(log_txs)
        .launch()
        .await?)
}
