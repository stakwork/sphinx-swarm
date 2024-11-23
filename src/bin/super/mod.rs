mod auth_token;
mod aws_util;
mod checker;
mod cmd;
mod ec2;
mod lightning_bots;
mod route53;
mod routes;
mod state;
mod util;

use cmd::{AddSwarmResponse, SuperSwarmResponse};
use cmd::{Cmd, SwarmCmd};
use lightning_bots::{change_lightning_bot_label, get_lightning_bots_details};
use sphinx_swarm::utils::getenv;
use state::RemoteStack;
use state::Super;
use util::{
    accessing_child_container_controller, add_new_swarm_details, add_new_swarm_from_child_swarm,
    get_aws_instance_types, get_child_swarm_config, get_child_swarm_containers,
    get_child_swarm_image_versions, get_config, get_swarm_instance_type, update_aws_instance_type,
    update_swarm_child_password,
};

use crate::checker::swarm_checker;
use crate::util::create_swarm_ec2;
use anyhow::{anyhow, Context, Result};
use rocket::tokio;
use routes::launch_rocket;
use sphinx_swarm::config::Role;
use sphinx_swarm::utils;
use sphinx_swarm::{auth, events, logs, rocket_utils::CmdRequest};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[rocket::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    sphinx_swarm::utils::setup_logs();

    let project = "super";
    let s: state::Super = load_config_file(project).await.expect("YAML CONFIG FAIL");
    println!("SUPER!!! {:?}", s);

    sphinx_swarm::auth::set_jwt_key(&s.jwt_key);

    state::hydrate(s).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();
    let log_txs = Arc::new(Mutex::new(log_txs));

    spawn_super_handler(project, rx);

    let cron_handler_res = swarm_checker().await;
    if let Err(e) = cron_handler_res {
        log::error!("CRON failed {:?}", e);
    }

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);

    let event_tx = events::new_event_chan();

    let _r = launch_rocket(tx.clone(), log_txs, event_tx).await?;

    Ok(())
}

pub async fn load_config_file(project: &str) -> Result<Super> {
    let yaml_path = format!("vol/{}/config.yaml", project);
    let s = utils::load_yaml(&yaml_path, Default::default()).await?;
    Ok(s)
}

pub async fn put_config_file(project: &str, rs: &Super) {
    let path = format!("vol/{}/config.yaml", project);
    utils::put_yaml(&path, rs).await;
}

fn access(cmd: &Cmd, state: &Super, user_id: &Option<u32>) -> bool {
    // login needs no auth
    match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::Login(_) => return true,
            SwarmCmd::SetChildSwarm(info) => {
                //get x-super-token
                let token = getenv("SUPER_TOKEN").unwrap_or("".to_string());
                if token.is_empty() {
                    return false;
                }
                if token != info.token {
                    return false;
                }
                return true;
            }
            _ => {}
        },
    }
    // user id required if not SwarmCmd::Login
    if user_id.is_none() {
        return false;
    }
    let user_id = user_id.unwrap();
    let user = state.users.iter().find(|u| u.id == user_id);
    // user required
    if user.is_none() {
        return false;
    }

    return match user.unwrap().role {
        Role::Super => true,
        Role::Admin => false,
        Role::SubAdmin => false,
    };
}

// tag is the service name
pub async fn super_handle(
    proj: &str,
    cmd: Cmd,
    _tag: &str,
    user_id: &Option<u32>,
) -> Result<String> {
    // conf can be mutated in place
    let mut state = state::STATE.lock().await;
    // println!("STACK {:?}", stack);

    let mut must_save_stack = false;

    if !access(&cmd, &state, user_id) {
        return Err(anyhow!("access denied"));
    }

    let ret = match cmd {
        Cmd::Swarm(swarm_cmd) => match swarm_cmd {
            SwarmCmd::GetConfig => {
                let res = get_config(&mut state).await?;
                must_save_stack = true;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::Login(ld) => match state.users.iter().find(|u| u.username == ld.username) {
                Some(user) => {
                    if !bcrypt::verify(&ld.password, &user.pass_hash)? {
                        Some("".to_string())
                    } else {
                        let mut hm = HashMap::new();
                        hm.insert("token", auth::make_jwt(user.id)?);
                        Some(serde_json::to_string(&hm)?)
                    }
                }
                None => Some("".to_string()),
            },
            SwarmCmd::ChangePassword(cp) => {
                match state.users.iter().position(|u| u.id == cp.user_id) {
                    Some(ui) => {
                        let old_pass_hash = &state.users[ui].pass_hash;
                        if bcrypt::verify(&cp.old_pass, old_pass_hash)? {
                            state.users[ui].pass_hash =
                                bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                            must_save_stack = true;
                            let mut hm = HashMap::new();
                            hm.insert("success", true);
                            Some(serde_json::to_string(&hm)?)
                        } else {
                            Some("".to_string())
                        }
                    }
                    None => Some("".to_string()),
                }
            }
            SwarmCmd::AddNewSwarm(swarm) => {
                let swarm_detail = RemoteStack {
                    host: swarm.host,
                    user: Some("".to_string()),
                    pass: Some("".to_string()),
                    ec2: Some(swarm.instance),
                    note: Some(swarm.description),
                    default_host: "".to_string(),
                    ec2_instance_id: "".to_string(),
                };

                let hm = add_new_swarm_details(&mut state, swarm_detail, &mut must_save_stack);

                Some(serde_json::to_string(&hm)?)
            }
            SwarmCmd::UpdateSwarm(swarm) => {
                let hm: AddSwarmResponse;
                match state.stacks.iter().position(|u| u.host == swarm.id) {
                    Some(ui) => {
                        state.stacks[ui] = RemoteStack {
                            host: swarm.host,
                            ec2: Some(swarm.instance),
                            note: Some(swarm.description),
                            user: state.stacks[ui].user.clone(),
                            pass: state.stacks[ui].pass.clone(),
                            default_host: state.stacks[ui].default_host.clone(),
                            ec2_instance_id: state.stacks[ui].ec2_instance_id.clone(),
                        };
                        must_save_stack = true;
                        hm = AddSwarmResponse {
                            success: true,
                            message: "Swarm updated successfully".to_string(),
                        };
                    }
                    None => {
                        hm = AddSwarmResponse {
                            success: false,
                            message: "swarm does not exist".to_string(),
                        };
                    }
                }

                Some(serde_json::to_string(&hm)?)
            }
            SwarmCmd::DeleteSwarm(swarm) => {
                let mut hm = HashMap::new();
                match state.delete_swarm_by_host(&swarm.host) {
                    Ok(()) => {
                        must_save_stack = true;
                        hm.insert("success", "true".to_string());
                        hm.insert("message", "Swarm deleted successfully".to_string());
                    }
                    Err(msg) => {
                        hm.insert("message", msg.clone());
                        hm.insert("success", "false".to_string());
                    }
                }
                Some(serde_json::to_string(&hm)?)
            }
            SwarmCmd::SetChildSwarm(c) => {
                let swarm_details = RemoteStack {
                    host: c.host,
                    note: Some("".to_string()),
                    pass: Some(c.password),
                    user: Some(c.username),
                    ec2: Some("".to_string()),
                    default_host: c.default_host,
                    ec2_instance_id: "".to_string(),
                };
                let hm =
                    add_new_swarm_from_child_swarm(&mut state, swarm_details, &mut must_save_stack);

                Some(serde_json::to_string(&hm)?)
            }
            SwarmCmd::GetChildSwarmConfig(info) => {
                let res: SuperSwarmResponse;
                //find node
                match state.find_swarm_by_host(&info.host) {
                    Some(swarm) => match get_child_swarm_config(&swarm).await {
                        Ok(result) => res = result,
                        Err(err) => {
                            res = SuperSwarmResponse {
                                success: false,
                                message: err.to_string(),
                                data: None,
                            }
                        }
                    },
                    None => {
                        res = SuperSwarmResponse {
                            success: false,
                            message: "Swarm does not exist".to_string(),
                            data: None,
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetChildSwarmContainers(info) => {
                let res: SuperSwarmResponse;
                match state.find_swarm_by_host(&info.host) {
                    Some(swarm) => match get_child_swarm_containers(&swarm).await {
                        Ok(result) => res = result,
                        Err(err) => {
                            res = SuperSwarmResponse {
                                success: false,
                                message: err.to_string(),
                                data: None,
                            }
                        }
                    },
                    None => {
                        res = SuperSwarmResponse {
                            success: false,
                            message: "Swarm does not exist".to_string(),
                            data: None,
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::StopChildSwarmContainers(info) => {
                let res = accessing_child_container_controller(&state, info, "StopContainer").await;

                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::StartChildSwarmContainers(info) => {
                let res =
                    accessing_child_container_controller(&state, info, "StartContainer").await;

                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::RestartChildSwarmContainers(info) => {
                let res =
                    accessing_child_container_controller(&state, info, "RestartContainer").await;

                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateChildSwarmContainers(info) => {
                let res = accessing_child_container_controller(&state, info, "UpdateNode").await;

                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetAwsInstanceTypes => {
                let res = get_aws_instance_types();
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::CreateNewEc2Instance(info) => {
                let res: SuperSwarmResponse;
                match create_swarm_ec2(&info, &mut state).await {
                    Ok(_) => {
                        must_save_stack = true;
                        res = SuperSwarmResponse {
                            success: true,
                            message: format!("{} was created successfully", &info.name.clone()),
                            data: None,
                        }
                    }
                    Err(err) => {
                        res = SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        }
                    }
                }

                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateAwsInstanceType(info) => {
                let res: SuperSwarmResponse;
                match update_aws_instance_type(info, &mut state).await {
                    Ok(_) => {
                        must_save_stack = true;
                        res = SuperSwarmResponse {
                            success: true,
                            message: "Instance updated successfully".to_string(),
                            data: None,
                        }
                    }
                    Err(err) => {
                        res = SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetInstanceType(info) => {
                let res: SuperSwarmResponse;
                match get_swarm_instance_type(info, &state) {
                    Ok(result) => res = result,
                    Err(err) => {
                        res = SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetSwarmChildImageVersions(info) => {
                let res: SuperSwarmResponse;
                match state.find_swarm_by_host(&info.host) {
                    Some(swarm) => match get_child_swarm_image_versions(&swarm).await {
                        Ok(result) => res = result,
                        Err(err) => {
                            res = SuperSwarmResponse {
                                success: false,
                                message: err.to_string(),
                                data: None,
                            }
                        }
                    },
                    None => {
                        res = SuperSwarmResponse {
                            success: false,
                            message: "Swarm does not exist".to_string(),
                            data: None,
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::ChangeChildSwarmPassword(info) => {
                let res: SuperSwarmResponse = update_swarm_child_password(info, &state).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetLightningBotsDetails => {
                let res: SuperSwarmResponse = get_lightning_bots_details(&state).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::ChangeLightningBotLabel(info) => {
                let res: SuperSwarmResponse =
                    change_lightning_bot_label(&mut state, &mut must_save_stack, info).await;
                Some(serde_json::to_string(&res)?)
            }
        },
    };

    if must_save_stack {
        put_config_file(proj, &state).await;
    }
    Ok(ret.context("internal error")?)
}

pub fn spawn_super_handler(proj: &str, mut rx: mpsc::Receiver<CmdRequest>) {
    let project = proj.to_string();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(cmd) = serde_json::from_str::<Cmd>(&msg.message) {
                match super_handle(&project, cmd, &msg.tag, &msg.user_id).await {
                    Ok(res) => {
                        let _ = msg.reply_tx.send(res);
                    }
                    Err(err) => {
                        msg.reply_tx
                            .send(fmt_err(&err.to_string()))
                            .expect("couldnt send cmd reply");
                    }
                }
            } else {
                msg.reply_tx
                    .send(fmt_err("Invalid Command"))
                    .expect("couldnt send cmd reply");
            }
        }
    });
}

fn fmt_err(err: &str) -> String {
    format!("{{\"stack_error\":\"{}\"}}", err.to_string())
}
