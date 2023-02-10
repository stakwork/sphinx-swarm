use anyhow::{anyhow, Result};
use bollard::Docker;
use images::lnd::{to_lnd_network, LndImage};
use images::relay::RelayImage;
use sphinx_swarm::config::Clients;
use sphinx_swarm::conn::lnd::utils::{dl_cert, dl_macaroon};
use sphinx_swarm::conn::lnd::{lndrpc::LndRPC};
use sphinx_swarm::conn::relay::RelayAPI;
use sphinx_swarm::images;
use sphinx_swarm::secrets;
use sphinx_swarm::utils::sleep_ms;
use sphinx_swarm::conn::lnd::utils::try_unlock_lnd;

// returns LndRPC client and address if test mine needed
pub async fn lnd_clients(
    docker: &Docker,
    proj: &str,
    lnd_node: &LndImage,
) -> Result<(LndRPC, Option<String>)> {
    let cert_path = "/home/.lnd/tls.cert";
    let cert = dl_cert(docker, &lnd_node.name, cert_path).await?;
    try_unlock_lnd(&cert, proj, lnd_node).await?;
    let netwk = to_lnd_network(lnd_node.network.as_str());
    let macpath = format!("/home/.lnd/data/chain/bitcoin/{}/admin.macaroon", netwk);
    let mac = dl_macaroon(docker, &lnd_node.name, &macpath).await?;
    let mut client = LndRPC::new(lnd_node, &cert, &mac)
        .await
        .map_err(|e| anyhow!("LndRPC::new failed: {}", e))?;
    if &lnd_node.network != "regtest" {
        return Ok((client, None));
    }
    let bal = client.try_get_balance().await?;
    if bal.confirmed_balance > 0 {
        return Ok((client, None));
    }
    let addy = client.new_address().await?;
    Ok((client, Some(addy.address)))
}

pub fn test_mine_if_needed(test_mine_addy: Option<String>, btc_name: &str, clients: &mut Clients) {
    if let Some(addy) = test_mine_addy {
        log::info!("mining 101 blocks to LND address {}", addy);
        if let Some(btcrpc) = clients.bitcoind.get(btc_name) {
            if let Err(e) = btcrpc.test_mine(101, Some(addy)) {
                log::error!("failed to test mine {}", e);
            } else {
                log::info!("blocks mined!");
            }
        }
    }
}

pub async fn relay_client(proj: &str, relay: &RelayImage) -> Result<RelayAPI> {
    let secs = secrets::load_secrets(proj).await;
    let relay_token = match secs.get(&relay.name) {
        Some(token) => token.clone(),
        None => secrets::random_word(12),
    };
    let api = RelayAPI::new(&relay, &relay_token, false).await?;
    let has_admin = api.try_has_admin().await?.response;
    if has_admin.unwrap_or(false) {
        log::info!("relay admin exists already");
        return Ok(api);
    }
    sleep_ms(2400).await;
    let root_pubkey = api.initial_admin_pubkey().await?;
    let claim_res = api.claim_user(&root_pubkey, &relay_token).await?;
    secrets::add_to_secrets(proj, &relay.name, &relay_token).await;
    println!(
        "Relay Root User claimed! {}",
        claim_res.response.unwrap().id
    );
    Ok(api)
}