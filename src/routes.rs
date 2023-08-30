use crate::auth;
use crate::cmd::{ChangePasswordInfo, Cmd, LoginInfo, SwarmCmd};
use crate::events::{get_event_tx, EventChans};
use crate::logs::{get_log_tx, LogChans, LOGS};
use crate::rocket_utils::{CmdRequest, Error, Result, CORS};
use fs::{relative, FileServer};
use response::stream::{Event, EventStream};
use rocket::serde::{
    json::{json, Json},
    Deserialize, Serialize,
};
use rocket::*;
use std::sync::Arc;
use tokio::sync::{broadcast::error::RecvError, mpsc, Mutex};

pub async fn launch_rocket(
    tx: mpsc::Sender<CmdRequest>,
    log_txs: Arc<Mutex<LogChans>>,
    event_txs: Arc<Mutex<EventChans>>,
) -> Result<Rocket<Ignite>> {
    Ok(rocket::build()
        .mount("/", FileServer::from(relative!("app/dist")))
        .mount(
            "/api/",
            routes![
                cmd,
                logs,
                logstream,
                login,
                refresh_jwt,
                all_options,
                update_password,
                events
            ],
        )
        .attach(CORS)
        .manage(tx)
        .manage(log_txs)
        .manage(event_txs)
        .launch()
        .await?)
}

#[options("/<_..>")]
pub fn all_options() {
    /* Intentionally left empty */
}

#[get("/cmd?<tag>&<txt>")]
pub async fn cmd(
    sender: &State<mpsc::Sender<CmdRequest>>,
    tag: &str,
    txt: &str,
    _claims: auth::AdminJwtClaims,
) -> Result<String> {
    let (request, reply_rx) = CmdRequest::new(tag, txt);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    Ok(reply)
}

#[get("/events?<tag>")]
pub async fn events(
    event_txs: &State<Arc<Mutex<EventChans>>>,
    mut end: Shutdown,
    tag: &str,
) -> EventStream![] {
    let event_tx = get_event_tx(tag, event_txs).await;
    let mut rx = event_tx.subscribe();
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

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginData {
    pub username: String,
    pub password: String,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginResult {
    pub token: String,
}

#[rocket::post("/login", data = "<body>")]
pub async fn login(
    sender: &State<mpsc::Sender<CmdRequest>>,
    body: Json<LoginData>,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
        username: body.username.clone(),
        password: body.password.clone(),
    }));
    let txt = serde_json::to_string(&cmd)?;
    let (request, reply_rx) = CmdRequest::new("SWARM", &txt);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    // empty string means unauthorized
    if reply.len() == 0 {
        return Err(Error::Unauthorized);
    }
    Ok(reply)
}

#[rocket::get("/refresh_jwt")]
pub async fn refresh_jwt(claims: auth::AdminJwtClaims) -> Result<Json<LoginResult>> {
    Ok(Json(LoginResult {
        token: auth::make_jwt(claims.user)?,
    }))
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdatePasswordData {
    pub old_pass: String,
    pub password: String,
}
#[rocket::put("/admin/password", data = "<body>")]
pub async fn update_password(
    sender: &State<mpsc::Sender<CmdRequest>>,
    body: Json<UpdatePasswordData>,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::ChangePassword(ChangePasswordInfo {
        user_id: claims.user,
        old_pass: body.old_pass.clone(),
        password: body.password.clone(),
    }));
    let txt = serde_json::to_string(&cmd)?;
    let (request, reply_rx) = CmdRequest::new("SWARM", &txt);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    // empty string means unauthorized
    if reply.len() == 0 {
        return Err(Error::Unauthorized);
    }
    Ok(reply)
}
