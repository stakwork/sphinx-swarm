use crate::env::check_env;
use crate::logs::{get_log_tx, LogChans, LOGS};
use fairing::{Fairing, Info, Kind};
use fs::{relative, FileServer};
use response::stream::{Event, EventStream};
use rocket::serde::json::json;
use rocket::*;
use std::sync::Arc;
use tokio::sync::{broadcast::error::RecvError, mpsc, oneshot, Mutex};

pub type Result<T> = std::result::Result<T, Error>;

/// Responses are received on the oneshot sender
#[derive(Debug)]
pub struct CmdRequest {
    pub tag: String,
    pub message: String,
    pub reply_tx: oneshot::Sender<String>,
}
impl CmdRequest {
    pub fn new(tag: &str, message: &str) -> (Self, oneshot::Receiver<String>) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cr = CmdRequest {
            tag: tag.to_string(),
            message: message.to_string(),
            reply_tx,
        };
        (cr, reply_rx)
    }
}

#[get("/cmd?<tag>&<txt>")]
pub async fn cmd(sender: &State<mpsc::Sender<CmdRequest>>, tag: &str, txt: &str) -> Result<String> {
    let final_txt = check_env(tag, txt).await;
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
async fn logs(tag: &str) -> Result<String> {
    let lgs = LOGS.lock().await;
    let ret = lgs.get(tag).unwrap_or(&Vec::new()).clone();
    Ok(json!(ret).to_string())
}

#[get("/logstream?<tag>")]
async fn logstream(
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

pub async fn launch_rocket(
    tx: mpsc::Sender<CmdRequest>,
    log_txs: Arc<Mutex<LogChans>>,
) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/", FileServer::from(relative!("app/public")))
        .mount("/api/", routes![cmd, logstream, logs])
        .attach(CORS)
        .manage(tx)
        .manage(log_txs)
        .launch()
        .await?)
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed")]
    Fail,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hex error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("rocket error: {0}")]
    Rocket(#[from] rocket::Error),
}

use rocket::http::Status;
use rocket::response::{self, Responder};
impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);
        match self {
            // in our simplistic example, we're happy to respond with the default 500 responder in all cases
            _ => Status::InternalServerError.respond_to(req),
        }
    }
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(http::Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(http::Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(http::Header::new(
            "Access-Control-Allow-Credentials",
            "true",
        ));
    }
}
