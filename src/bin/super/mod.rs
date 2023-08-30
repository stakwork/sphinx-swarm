mod state;
use state::Super;

use anyhow::{Context, Result};
use rocket::tokio;
use serde::{Deserialize, Serialize};
use sphinx_swarm::routes;
use sphinx_swarm::utils;
use sphinx_swarm::{events, logs, rocket_utils::CmdRequest};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[rocket::main]
async fn main() -> Result<()> {
    let project = "super";
    let s: state::Super = load_config_file(project).await.expect("YAML CONFIG FAIL");
    println!("SUPER! {:?}", s);

    sphinx_swarm::auth::set_jwt_key(&s.jwt_key);

    state::hydrate(s).await;

    let (tx, rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();
    let log_txs = Arc::new(Mutex::new(log_txs));

    spawn_super_handler(project, rx);

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);

    let event_txs = events::new_event_chans();
    let event_txs = Arc::new(Mutex::new(event_txs));

    let _r = routes::launch_rocket(tx.clone(), log_txs, event_txs).await?;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SuperCmd {
    GetConfigs,
}

// tag is the service name
pub async fn super_handle(proj: &str, cmd: SuperCmd, tag: &str) -> Result<String> {
    // conf can be mutated in place
    let mut state = state::STATE.lock().await;
    // println!("STACK {:?}", stack);

    let mut must_save_stack = false;

    let ret: Option<String> = match cmd {
        SuperCmd::GetConfigs => {
            let res = &state.remove_tokens();
            Some(serde_json::to_string(&res)?)
        }
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
            if let Ok(cmd) = serde_json::from_str::<SuperCmd>(&msg.message) {
                match super_handle(&project, cmd, &msg.tag).await {
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
