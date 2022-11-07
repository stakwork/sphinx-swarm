use crate::cmd::{Cmd, SwarmCmd};
use crate::config::{Config, CONFIG};
use anyhow::Result;

// tag is the service name
pub async fn handle(cmd: Cmd, tag: &str) -> Result<String> {
    // conf can be mutated in place
    let mut conf = CONFIG.lock().await;
    println!("CONF {:?}", conf);
    println!("CMD {:?}", cmd);

    let ret: Option<String> = match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::GetConfig => Some(serde_json::to_string(&*conf)?),
            _ => None,
        },
        Cmd::Relay(c) => None,
    };
    match ret {
        Some(r) => Ok(r),
        None => Err(anyhow::anyhow!("no return value".to_string())),
    }
}
