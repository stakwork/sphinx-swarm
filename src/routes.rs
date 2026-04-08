use crate::app_login;
use crate::auth;
use crate::cmd::SignUpAdminPubkeyDetails;
use crate::cmd::UpdateAdminPubkeyInfo;
use crate::cmd::{ChangeAdminInfo, ChangePasswordInfo, Cmd, LoginInfo, SwarmCmd};
use crate::dock::dockr;
use crate::events::{get_event_tx, EventChan};
use crate::handler;
use crate::logs::{get_log_tx, LogChans, LOGS};
use crate::rocket_utils::{Error, Result, CORS};
use fs::{relative, FileServer};
use response::stream::{Event, EventStream};
use rocket::serde::{
    json::{json, Json},
    Deserialize, Serialize,
};
use rocket::*;
use sphinx_auther::secp256k1::PublicKey;
use std::sync::Arc;
use tokio::sync::{broadcast::error::RecvError, Mutex};

/// Managed state: the project name (e.g. "stack")
pub struct ProjectName(pub String);

pub async fn launch_rocket(
    proj: String,
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
                verify_challenge_token,
                get_challenge,
                update_admin_pubkey,
                check_challenge,
                get_signup_challenge,
                check_signup_challenge,
                service_health,
            ],
        )
        .attach(CORS)
        .manage(ProjectName(proj))
        .manage(log_txs)
        .manage(event_tx)
        .launch()
        .await?)
}

#[options("/<_..>")]
pub fn all_options() {
    /* Intentionally left empty */
}

fn fmt_err(err: &str) -> String {
    format!("{{\"stack_error\":\"{}\"}}", err)
}

#[get("/cmd?<tag>&<txt>")]
pub async fn cmd(
    proj: &State<ProjectName>,
    tag: &str,
    txt: &str,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let cmd: Cmd = serde_json::from_str(txt).map_err(|_| Error::Fail)?;
    let docker = dockr();
    match handler::handle(&proj.0, cmd, tag, &docker, &Some(claims.user)).await {
        Ok(res) => Ok(res),
        Err(err) => Ok(fmt_err(&err.to_string())),
    }
}

#[get("/events")]
pub async fn events(event_tx: &State<Arc<Mutex<EventChan>>>, mut end: Shutdown) -> EventStream![] {
    let event_tx = get_event_tx(event_tx).await;
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

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct VerifyTokenResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ChallengeStatusResponse {
    pub success: bool,
    pub token: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ServiceHealthResponse {
    pub status: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GetChallengeResponse {
    pub success: bool,
    pub challenge: String,
}

#[rocket::post("/login", data = "<body>")]
pub async fn login(
    proj: &State<ProjectName>,
    body: Json<LoginData>,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
        username: body.username.clone(),
        password: body.password.clone(),
    }));
    let docker = dockr();
    match handler::handle(&proj.0, cmd, "SWARM", &docker, &None).await {
        Ok(reply) => {
            if reply.is_empty() {
                Err(Error::Unauthorized)
            } else {
                Ok(reply)
            }
        }
        Err(err) => Ok(fmt_err(&err.to_string())),
    }
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
    proj: &State<ProjectName>,
    body: Json<UpdatePasswordData>,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::ChangePassword(ChangePasswordInfo {
        user_id: claims.user,
        old_pass: body.old_pass.clone(),
        password: body.password.clone(),
    }));
    let docker = dockr();
    match handler::handle(&proj.0, cmd, "SWARM", &docker, &Some(claims.user)).await {
        Ok(reply) => {
            if reply.is_empty() {
                Err(Error::Unauthorized)
            } else {
                Ok(reply)
            }
        }
        Err(err) => Ok(fmt_err(&err.to_string())),
    }
}

#[get("/challenge")]
pub async fn get_challenge() -> Result<Json<GetChallengeResponse>> {
    let challenge = app_login::generate_challenge().await;
    Ok(Json(GetChallengeResponse {
        success: true,
        challenge: challenge,
    }))
}

#[get("/signup_challenge")]
pub async fn get_signup_challenge(
    claims: auth::AdminJwtClaims,
) -> Result<Json<GetChallengeResponse>> {
    let challenge = app_login::generate_signup_challenge(claims.user).await;
    Ok(Json(GetChallengeResponse {
        success: true,
        challenge: challenge,
    }))
}

#[post("/verify/<challenge>?<token>")]
pub async fn verify_challenge_token(
    challenge: &str,
    token: &str,
) -> Result<Json<VerifyTokenResponse>> {
    let verify = app_login::verify_signed_token(challenge, token).await?;
    Ok(Json(VerifyTokenResponse {
        success: verify.success,
        message: verify.message,
    }))
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateAdminPubkeyData {
    pub pubkey: PublicKey,
}

#[rocket::put("/admin/pubkey", data = "<body>")]
pub async fn update_admin_pubkey(
    proj: &State<ProjectName>,
    body: Json<UpdateAdminPubkeyData>,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::UpdateAdminPubkey(UpdateAdminPubkeyInfo {
        user_id: claims.user,
        pubkey: body.pubkey.clone(),
    }));
    let docker = dockr();
    match handler::handle(&proj.0, cmd, "SWARM", &docker, &Some(claims.user)).await {
        Ok(reply) => {
            if reply.is_empty() {
                Err(Error::Unauthorized)
            } else {
                Ok(reply)
            }
        }
        Err(err) => Ok(fmt_err(&err.to_string())),
    }
}

#[get("/poll/<challenge>")]
pub async fn check_challenge(challenge: &str) -> Result<Json<ChallengeStatusResponse>> {
    let response = app_login::check_challenge_status(challenge).await?;

    Ok(Json(ChallengeStatusResponse {
        success: response.success,
        token: response.token,
        message: response.message,
    }))
}

#[get("/poll_signup_challenge/<challenge>?<username>")]
pub async fn check_signup_challenge(
    proj: &State<ProjectName>,
    challenge: &str,
    username: String,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::SignUpAdminPubkey(SignUpAdminPubkeyDetails {
        user_id: claims.user,
        challenge: challenge.to_string(),
        username: username.to_string(),
    }));
    let docker = dockr();
    match handler::handle(&proj.0, cmd, "SWARM", &docker, &Some(claims.user)).await {
        Ok(reply) => {
            if reply.is_empty() {
                Err(Error::Unauthorized)
            } else {
                Ok(reply)
            }
        }
        Err(err) => Ok(fmt_err(&err.to_string())),
    }
}

// /health
#[get("/health")]
pub async fn service_health() -> Result<Json<ServiceHealthResponse>> {
    Ok(Json(ServiceHealthResponse {
        status: "ok".to_string(),
    }))
}
