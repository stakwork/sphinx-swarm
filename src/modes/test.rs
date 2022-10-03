use crate::dock::*;
use crate::images;
use anyhow::Result;
use bollard::Docker;

pub async fn run(docker: Docker) -> Result<()> {
    let btc1 = images::btc("bitcoind", "regtest");
    let _id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");
    let logs = container_logs(&docker, "bitcoind").await;
    log::info!("LOGS {:?}", logs);
    Ok(())
}
