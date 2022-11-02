use crate::env::check_env;
use crate::logs::{get_log_tx, LogChans, LOGS};
use crate::rocket_utils::{Error, Result, *};
use fs::{relative, FileServer};
use response::stream::{Event, EventStream};
use rocket::serde::json::json;
use rocket::*;
use std::sync::Arc;
use tokio::sync::{broadcast::error::RecvError, mpsc, Mutex};

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

#[get("/logs?<tag>")]
pub async fn logs(tag: &str) -> Result<String> {
    let lgs = LOGS.lock().await;
    let ret = lgs.get(tag).unwrap_or(&Vec::new()).clone();
    Ok(json!(ret).to_string())
}

#[get("/logstream?<tag>")]
pub async fn logstream(
    log_txs: &State<Arc<Mutex<LogChans>>>,
    mut end: Shutdown,
    tag: &str,
) -> EventStream![] {
    let log_tx = get_log_tx(tag, log_txs).await;
    let mut rx = log_tx.subscribe();
    EventStream! {
        loop {
            let msg = tokio::select! {
                msg = rx.recv() => match msg {
                    Ok(lo) => lo,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}
