use crate::logs::LogChans;
use crate::rocket_utils::*;
use crate::routes::{cmd, logs, logstream};
use fs::{relative, FileServer};
use rocket::*;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub async fn launch_rocket(
    tx: mpsc::Sender<CmdRequest>,
    log_txs: Arc<Mutex<LogChans>>,
) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/", FileServer::from(relative!("src/cmd/stack/app/public")))
        .mount("/api/", routes![cmd, logs, logstream])
        .attach(CORS)
        .manage(tx)
        .manage(log_txs)
        .launch()
        .await?)
}
