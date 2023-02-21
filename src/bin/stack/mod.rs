mod builder;
mod handler;
mod setup;
mod srv;
mod update;

use anyhow::Result;
use rocket::tokio;
use sphinx_swarm::cmd::Cmd;
use sphinx_swarm::config::{load_config_file, put_config_file, Stack, State, STATE};
use sphinx_swarm::{dock::*, logs, rocket_utils::CmdRequest};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[rocket::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let proj = "stack";
    let stack: Stack = load_config_file(proj).await;
    let clients = builder::build_stack(proj, &docker, &stack).await?;
    put_config_file(proj, &stack).await;
    // put the jwt key into a var
    sphinx_swarm::auth::set_jwt_key(&stack.jwt_key);
    // set into the main state mutex
    let mut state = STATE.lock().await;
    *state = State { stack, clients };
    // drop it immediately
    drop(state);

    let (tx, mut rx) = mpsc::channel::<CmdRequest>(1000);
    let log_txs = logs::new_log_chans();

    let docker2 = docker.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(cmd) = serde_json::from_str::<Cmd>(&msg.message) {
                match handler::handle(proj, cmd, &msg.tag, &docker2).await {
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

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);
    let log_txs = Arc::new(Mutex::new(log_txs));
    let _r = srv::launch_rocket(tx.clone(), log_txs).await?;

    // for (_, id) in ids {
    //     stop_and_remove(&docker, &id).await?;
    // }

    Ok(())
}

fn fmt_err(err: &str) -> String {
    format!("{{\"stack_error\":\"{}\"}}", err.to_string())
}
