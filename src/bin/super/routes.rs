use crate::auth;
use crate::auth_token::VerifySuperToken;
use crate::cmd::{
    AddSwarmResponse, ChildSwarm, Cmd, CreateEc2InstanceInfo, GetChildSwarmCredentialsReq,
    StopEc2InstanceInfo, SuperSwarmResponse, SwarmCmd,
};
use crate::events::EventChan;
use crate::logs::LogChans;
use crate::service::check_domain::check_domain;
use crate::util::{get_child_swarm_credentials, get_swarm_details_by_id};
use crate::cmd::{ChangePasswordInfo, LoginInfo};
use crate::{fmt_err, super_handle};
use fs::{relative, FileServer};
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use rocket::*;
use sphinx_swarm::config::{
    ApiResponse, SendSwarmDetailsBody, SendSwarmDetailsResponse, UpdateChildSwarmPublicIpBody,
};
use sphinx_swarm::rocket_utils::{Error, Result, CORS};
use sphinx_swarm::routes::{
    all_options, events, logs, logstream, refresh_jwt, LoginData, UpdatePasswordData,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Managed state: the project name (e.g. "super")
pub struct SuperProjectName(pub String);

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
                super_login,
                refresh_jwt,
                all_options,
                super_update_password,
                events,
                add_new_swarm,
                create_new_swarm,
                stop_swarm,
                get_swarm_details,
                check_duplicate_domain,
                update_child_swarm_public_ip,
                get_swarm_credentials
            ],
        )
        .attach(CORS)
        .manage(SuperProjectName(proj))
        .manage(log_txs)
        .manage(event_tx)
        .launch()
        .await?)
}

#[get("/cmd?<tag>&<txt>")]
async fn cmd(
    proj: &State<SuperProjectName>,
    tag: &str,
    txt: &str,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let cmd: Cmd = serde_json::from_str(txt).map_err(|_| Error::Fail)?;
    match super_handle(&proj.0, cmd, tag, &Some(claims.user)).await {
        Ok(res) => Ok(res),
        Err(err) => Ok(fmt_err(&err.to_string())),
    }
}

// Super's own login route (calls super_handle instead of stack's handle)
#[rocket::post("/login", data = "<body>")]
async fn super_login(
    proj: &State<SuperProjectName>,
    body: Json<LoginData>,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
        username: body.username.clone(),
        password: body.password.clone(),
    }));
    match super_handle(&proj.0, cmd, "SWARM", &None).await {
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

// Super's own update_password route
#[rocket::put("/admin/password", data = "<body>")]
async fn super_update_password(
    proj: &State<SuperProjectName>,
    body: Json<UpdatePasswordData>,
    claims: auth::AdminJwtClaims,
) -> Result<String> {
    let cmd: Cmd = Cmd::Swarm(SwarmCmd::ChangePassword(ChangePasswordInfo {
        user_id: claims.user,
        old_pass: body.old_pass.clone(),
        password: body.password.clone(),
    }));
    match super_handle(&proj.0, cmd, "SWARM", &Some(claims.user)).await {
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

#[rocket::post("/super/add_new_swarm", data = "<body>")]
async fn add_new_swarm(
    proj: &State<SuperProjectName>,
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

    let reply = match super_handle(&proj.0, cmd, "SWARM", &None).await {
        Ok(res) => res,
        Err(err) => return Ok(Custom(
            Status::InternalServerError,
            Json(SendSwarmDetailsResponse { message: err.to_string() }),
        )),
    };

    if reply.is_empty() {
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
    proj: &State<SuperProjectName>,
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
        enable_cloudwatch_alarms: body.enable_cloudwatch_alarms.clone(),
        workspace_type: body.workspace_type.clone(),
        owner_pubkey: body.owner_pubkey.clone(),
    }));

    let reply = match super_handle(&proj.0, cmd, "SWARM", &None).await {
        Ok(res) => res,
        Err(err) => return Ok(Custom(
            Status::InternalServerError,
            Json(SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            }),
        )),
    };

    if reply.is_empty() {
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
    proj: &State<SuperProjectName>,
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

    let reply = match super_handle(&proj.0, cmd, "SWARM", &None).await {
        Ok(res) => res,
        Err(err) => return Ok(Custom(
            Status::InternalServerError,
            Json(SuperSwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            }),
        )),
    };

    if reply.is_empty() {
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
    proj: &State<SuperProjectName>,
    body: Json<UpdateChildSwarmPublicIpBody>,
    verify_super_token: VerifySuperToken,
) -> Result<Custom<Json<ApiResponse>>> {
    if let None = verify_super_token.token {
        return Ok(Custom(
            Status::Unauthorized,
            Json(ApiResponse {
                success: false,
                message: "unauthorized, invalid token".to_string(),
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

    let reply = match super_handle(&proj.0, cmd, "SWARM", &None).await {
        Ok(res) => res,
        Err(err) => return Ok(Custom(
            Status::InternalServerError,
            Json(ApiResponse {
                success: false,
                message: err.to_string(),
            }),
        )),
    };

    if reply.is_empty() {
        return Err(Error::Unauthorized);
    }

    let response: SuperSwarmResponse = serde::json::from_str(reply.as_str())?;

    let mut status = Status::BadRequest;
    if response.success == true {
        status = Status::Ok
    }

    return Ok(Custom(
        status,
        Json(ApiResponse {
            success: response.success,
            message: response.message,
        }),
    ));
}

#[rocket::get("/super/swarm_credentials?<host>&<id>&<instance_id>&<is_reserved>")]
async fn get_swarm_credentials(
    host: Option<String>,
    id: Option<String>,
    instance_id: Option<String>,
    is_reserved: Option<bool>,
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

    let state = crate::state::STATE.read().await;
    let req = GetChildSwarmCredentialsReq {
        host,
        id,
        instance_id,
        is_reserved,
    };
    let response = get_child_swarm_credentials(req, &state);
    let status = if response.success {
        Status::Ok
    } else {
        Status::BadRequest
    };
    return Ok(Custom(status, Json(response)));
}
