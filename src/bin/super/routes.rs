use crate::auth;
use crate::auth_token::VerifySuperToken;
use crate::cmd::{
    AddSwarmResponse, ChildSwarm, Cmd, CreateEc2InstanceInfo, StopEc2InstanceInfo,
    SuperSwarmResponse, SwarmCmd,
};
use crate::events::EventChan;
use crate::logs::LogChans;
use crate::service::check_domain::check_domain;
use crate::util::get_swarm_details_by_id;
use fs::{relative, FileServer};
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use rocket::*;
use sphinx_swarm::config::{
    ApiResponse, SendSwarmDetailsBody, SendSwarmDetailsResponse, UpdateChildSwarmPublicIpBody,
};
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
                add_new_swarm,
                create_new_swarm,
                stop_swarm,
                get_swarm_details,
                check_duplicate_domain,
                update_child_swarm_public_ip
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
async fn cmd(
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
async fn add_new_swarm(
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
        default_host: body.default_host.clone(),
        id: body.id.clone(),
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

#[rocket::post("/super/new_swarm", data = "<body>")]
async fn create_new_swarm(
    sender: &State<mpsc::Sender<CmdRequest>>,
    body: Json<CreateEc2InstanceInfo>,
    verify_super_token: VerifySuperToken,
) -> Result<Custom<Json<SuperSwarmResponse>>> {
    if let None = verify_super_token.token {
        return Ok(Custom(
            Status::Unauthorized,
            Json(SuperSwarmResponse {
                success: false,
                message: "unauthorized, invalid token".to_string(),
                data: None,
            }),
        ));
    }

    let cmd: Cmd = Cmd::Swarm(SwarmCmd::CreateNewEc2Instance(CreateEc2InstanceInfo {
        name: body.name.clone(),
        vanity_address: body.vanity_address.clone(),
        instance_type: body.instance_type.clone(),
        token: verify_super_token.token.clone(),
        env: body.env.clone(),
        subdomain_ssl: body.subdomain_ssl.clone(),
        password: body.password.clone(),
        testing: body.testing.clone(),
    }));

    let txt = serde_json::to_string(&cmd)?;
    let (request, reply_rx) = CmdRequest::new("SWARM", &txt, None);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;

    // empty string means unauthorized
    if reply.len() == 0 {
        return Err(Error::Unauthorized);
    }

    let response: SuperSwarmResponse = serde::json::from_str(reply.as_str())?;

    let mut status = Status::Conflict;

    if response.success == true {
        status = Status::Created
    }

    return Ok(Custom(status, Json(response)));
}

#[rocket::post("/super/stop_swarm", data = "<body>")]
async fn stop_swarm(
    sender: &State<mpsc::Sender<CmdRequest>>,
    body: Json<StopEc2InstanceInfo>,
    verify_super_token: VerifySuperToken,
) -> Result<Custom<Json<SuperSwarmResponse>>> {
    if let None = verify_super_token.token {
        return Ok(Custom(
            Status::Unauthorized,
            Json(SuperSwarmResponse {
                success: false,
                message: "unauthorized, invalid token".to_string(),
                data: None,
            }),
        ));
    }

    let cmd: Cmd = Cmd::Swarm(SwarmCmd::StopEc2Instance(StopEc2InstanceInfo {
        instance_id: body.instance_id.clone(),
        token: verify_super_token.token.clone(),
    }));

    let txt = serde_json::to_string(&cmd)?;
    let (request, reply_rx) = CmdRequest::new("SWARM", &txt, None);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;

    // empty string means unauthorized
    if reply.len() == 0 {
        return Err(Error::Unauthorized);
    }

    let response: SuperSwarmResponse = serde::json::from_str(reply.as_str())?;

    let mut status = Status::BadRequest;

    if response.success == true {
        status = Status::Ok
    }

    return Ok(Custom(status, Json(response)));
}

#[rocket::get("/super/details?<id>")]
async fn get_swarm_details(
    id: &str,
    verify_super_token: VerifySuperToken,
) -> Result<Custom<Json<SuperSwarmResponse>>> {
    if let None = verify_super_token.token {
        return Ok(Custom(
            Status::Unauthorized,
            Json(SuperSwarmResponse {
                success: false,
                message: "unauthorized, invalid token".to_string(),
                data: None,
            }),
        ));
    }

    let response = get_swarm_details_by_id(id).await;
    let mut status = Status::Ok;

    if response.success != true {
        status = Status::BadRequest
    }

    return Ok(Custom(status, Json(response)));
}

#[rocket::get("/super/check-domain?<domain>")]
async fn check_duplicate_domain(
    domain: &str,
    verify_super_token: VerifySuperToken,
) -> Result<Custom<Json<SuperSwarmResponse>>> {
    if let None = verify_super_token.token {
        return Ok(Custom(
            Status::Unauthorized,
            Json(SuperSwarmResponse {
                success: false,
                message: "unauthorized, invalid token".to_string(),
                data: None,
            }),
        ));
    }

    let response = check_domain(domain).await;
    let mut status = Status::Ok;

    if response.success != true {
        status = Status::BadRequest
    }

    return Ok(Custom(status, Json(response)));
}

#[rocket::post("/super/update_child_public_ip", data = "<body>")]
async fn update_child_swarm_public_ip(
    sender: &State<mpsc::Sender<CmdRequest>>,
    body: Json<UpdateChildSwarmPublicIpBody>,
    verify_super_token: VerifySuperToken,
) -> Result<Custom<Json<ApiResponse>>> {
    if let None = verify_super_token.token {
        return Ok(Custom(
            Status::Unauthorized,
            Json(ApiResponse {
                message: "unauthorized, invalid token".to_string(),
                success: false,
            }),
        ));
    }

    let cmd: Cmd = Cmd::Swarm(SwarmCmd::UpdateChildSwarmPublicIp(
        UpdateChildSwarmPublicIpBody {
            public_ip: body.public_ip.clone(),
            token: verify_super_token.token.clone(),
            id: body.id.clone(),
        },
    ));

    let txt = serde_json::to_string(&cmd)?;
    let (request, reply_rx) = CmdRequest::new("SWARM", &txt, None);
    let _ = sender.send(request).await.map_err(|_| Error::Fail)?;
    let reply = reply_rx.await.map_err(|_| Error::Fail)?;

    // empty string means unauthorized
    if reply.len() == 0 {
        return Err(Error::Unauthorized);
    }

    let response: AddSwarmResponse = serde::json::from_str(reply.as_str())?;

    let mut status = Status::BadRequest;

    if response.success == true {
        status = Status::Ok
    }

    return Ok(Custom(
        status,
        Json(ApiResponse {
            message: response.message,
            success: response.success,
        }),
    ));
}
