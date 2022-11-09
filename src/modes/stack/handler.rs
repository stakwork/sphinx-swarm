use crate::cmd::Cmd;
use crate::config::CONFIG;

// tag is the service name
pub async fn handle(cmd: Cmd, tag: &str) {
    // conf can be mutated in place
    let mut conf = CONFIG.lock().await;
    println!("CONF {:?}", conf);
    println!("CMD {:?}", cmd);
}
// cargo run  down
