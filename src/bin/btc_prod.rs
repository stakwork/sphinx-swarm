use anyhow::Result;
use sphinx_swarm::dock::*;
use sphinx_swarm::images::btc::{btc, BtcImage};

#[rocket::main]
pub async fn main() -> Result<()> {
    let docker = dockr();
    sphinx_swarm::utils::setup_logs();

    let proj = "stack";
    let version = "v23.0";
    let network = "bitcoin";
    let img = BtcImage::new("bitcoind", version, network, "sphinx");
    log::info!("bitcoind rpc:");
    log::info!("==> user: sphinx ==> password: {}", &img.pass);
    let btc1 = btc(proj, &img);

    let btc_id = create_and_start(&docker, btc1).await?;
    log::info!("btc launched! {}", btc_id);
    Ok(())
}
