use anyhow::Result;

use crate::config::{State, STATE};

pub async fn handle_check_public_ip_via_cron() -> Result<()> {
    let mut state = STATE.lock().await;
    Ok(())
}
