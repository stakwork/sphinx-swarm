use anyhow::Result;
use sphinx_swarm::dock::*;
use sphinx_swarm::images;

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let btc_node = images::btc::BtcImage::new("bitcoind", "23.0", "regtest", "foo");
    let btc1 = images::btc::btc("test", &btc_node);
    let _id = create_and_start(&docker, btc1).await?;
    log::info!("created bitcoind");
    let logs = container_logs(&docker, "bitcoind").await;
    log::info!("LOGS {:?}", logs);
    Ok(())
}
