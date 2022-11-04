use crate::logs::LogChans;
use crate::rocket_utils::{CmdRequest, Error, Result, CORS};
use crate::routes::{cmd, logs, logstream};
use fs::{relative, FileServer};
use rocket::*;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[get("/asdf?<tag>&<txt>")]
pub async fn asdf(
    sender: &State<mpsc::Sender<CmdRequest>>,
    tag: &str,
    txt: &str,
) -> Result<String> {
    let (request, reply_rx) = CmdRequest::new(tag, &txt);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    Ok(reply)
}

pub async fn launch_rocket(
    tx: mpsc::Sender<CmdRequest>,
    log_txs: Arc<Mutex<LogChans>>,
) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount(
            "/",
            FileServer::from(relative!("src/modes/stack/app/public")),
        )
        .mount("/api/", routes![cmd, logs, logstream, asdf])
        .attach(CORS)
        .manage(tx)
        .manage(log_txs)
        .launch()
        .await?)
}
