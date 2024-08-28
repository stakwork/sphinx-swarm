mod checker;
mod state;
use state::RemoteStack;
use state::Super;

use crate::checker::swarm_checker;
use anyhow::{anyhow, Context, Result};
use rocket::tokio;
use sphinx_swarm::config::Role;
use sphinx_swarm::routes;
use sphinx_swarm::super_cmd::{Cmd, SwarmCmd};
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

    let _r = routes::launch_rocket(tx.clone(), log_txs, event_tx).await?;

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
    if let Cmd::Swarm(c) = cmd {
        if let SwarmCmd::Login(_) = c {
            return true;
        }
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
                let res = &state.remove_tokens();
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
                let mut hm = HashMap::new();
                match state.find_swarm_by_host(&swarm.host) {
                    Some(_swarm) => {
                        hm.insert("success", "false");
                        hm.insert("message", "swarm already exist");
                    }
                    None => {
                        let new_swarm = RemoteStack {
                            host: swarm.host,
                            note: Some(swarm.description),
                            ec2: Some(swarm.instance),
                            user: Some(swarm.username),
                            pass: Some(swarm.password),
                        };
                        state.add_remote_stack(new_swarm);
                        must_save_stack = true;
                        hm.insert("success", "true");
                        hm.insert("message", "Swarm added successfully");
                    }
                }
                Some(serde_json::to_string(&hm)?)
            }
            SwarmCmd::UpdateSwarm(swarm) => {
                let mut hm = HashMap::new();
                match state.stacks.iter().position(|u| u.host == swarm.id) {
                    Some(ui) => {
                        state.stacks[ui] = RemoteStack {
                            host: swarm.host,
                            ec2: Some(swarm.instance),
                            note: Some(swarm.description),
                            user: state.stacks[ui].user.clone(),
                            pass: state.stacks[ui].pass.clone(),
                        };
                        must_save_stack = true;
                        hm.insert("success", "true");
                        hm.insert("message", "Swarm updated successfully");
                    }
                    None => {
                        hm.insert("success", "false");
                        hm.insert("message", "swarm does not exist");
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
            SwarmCmd::SetChildSwarm(_c) => {
                // do the thing
                None
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
