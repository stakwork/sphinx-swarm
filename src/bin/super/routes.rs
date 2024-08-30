use crate::auth;
use crate::auth_token::VerifySuperToken;
use crate::cmd::{AddSwarmResponse, ChildSwarm, Cmd, SwarmCmd};
use crate::events::EventChan;
use crate::logs::LogChans;
use fs::{relative, FileServer};
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use rocket::*;
use sphinx_swarm::config::SendSwarmDetailsBody;
use sphinx_swarm::config::SendSwarmDetailsResponse;
use sphinx_swarm::rocket_utils::{CmdRequest, Error, Result, CORS};
use sphinx_swarm::routes::{
    all_options, events, login, logs, logstream, refresh_jwt, update_password,
};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub async fn launch_rocket(
    tx: mpsc::Sender<CmdRequest>,
    log_txs: Arc<Mutex<LogChans>>,
    event_tx: Arc<Mutex<EventChan>>,
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
                events,
                add_new_swarm
            ],
        )
        .attach(CORS)
        .manage(tx)
        .manage(log_txs)
        .manage(event_tx)
        .launch()
        .await?)
}

#[get("/cmd?<tag>&<txt>")]
pub async fn cmd(
    sender: &State<mpsc::Sender<CmdRequest>>,
    tag: &str,
    txt: &str,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let (request, reply_rx) = CmdRequest::new(tag, txt, Some(claims.user));
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;
    Ok(reply)
}

#[rocket::post("/super/add_new_swarm", data = "<body>")]
pub async fn add_new_swarm(
    sender: &State<mpsc::Sender<CmdRequest>>,
    body: Json<SendSwarmDetailsBody>,
    verify_super_token: VerifySuperToken,
) -> Result<Custom<Json<SendSwarmDetailsResponse>>> {
    if let None = verify_super_token.token {
        return Ok(Custom(
            Status::Unauthorized,
            Json(SendSwarmDetailsResponse {
                message: "unauthorized, invalid token".to_string(),
            }),
        ));
    }

    let cmd: Cmd = Cmd::Swarm(SwarmCmd::SetChildSwarm(ChildSwarm {
        host: body.host.clone(),
        username: body.username.clone(),
        password: body.password.clone(),
        token: verify_super_token.token.unwrap(),
    }));

    let txt = serde_json::to_string(&cmd)?;
    let (request, reply_rx) = CmdRequest::new("SWARM", &txt, None);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;

    // empty string means unauthorized
    if reply.len() == 0 {
        return Err(Error::Unauthorized);
    }

    let response: AddSwarmResponse = serde::json::from_str(reply.as_str())?;

    let mut status = Status::Conflict;

    if response.success == true {
        status = Status::Created
    }

    return Ok(Custom(
        status,
        Json(SendSwarmDetailsResponse {
            message: response.message,
        }),
    ));
}
