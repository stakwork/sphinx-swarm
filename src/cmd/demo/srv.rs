use crate::env::check_env;
use crate::logs::LogChans;
use crate::rocket_utils::{Error, Result, *};
use crate::routes::{logs, logstream};
use fs::{relative, FileServer};
use rocket::*;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[get("/cmd?<tag>&<txt>")]
pub async fn cmd(sender: &State<mpsc::Sender<CmdRequest>>, tag: &str, txt: &str) -> Result<String> {
    let (final_txt, skip) = check_env(tag, txt).await;
    if skip {
        // dont process the "export" cmd
        return Ok("".to_string());
    }
    let (request, reply_rx) = CmdRequest::new(tag, &final_txt);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    Ok(transform_reply(&reply))
}

fn transform_reply(reply: &str) -> String {
    let no_exec = "OCI runtime exec failed: exec failed: unable to start container process: exec:";
    if reply.starts_with(no_exec) {
        return reply.replace(no_exec, "").to_string();
    }
    reply.to_string()
}

pub async fn launch_rocket(
    tx: mpsc::Sender<CmdRequest>,
    log_txs: Arc<Mutex<LogChans>>,
) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/", FileServer::from(relative!("src/cmd/demo/app/public")))
        .mount("/api/", routes![cmd, logstream, logs])
        .attach(CORS)
        .manage(tx)
        .manage(log_txs)
        .launch()
        .await?)
}
