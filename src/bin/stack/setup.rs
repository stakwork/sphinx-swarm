use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use images::lnd::LndImage;
use images::relay::RelayImage;
use rocket::tokio;
use sphinx_swarm::config::Clients;
use sphinx_swarm::conn::lnd::{lndrpc::LndRPC, unlocker::LndUnlocker};
use sphinx_swarm::conn::relay::RelayAPI;
use sphinx_swarm::dock::download_from_container;
use sphinx_swarm::images;
use sphinx_swarm::secrets;
use sphinx_swarm::utils::domain;

// returns LndRPC client and address if test mine needed
pub async fn lnd_clients(
    docker: &Docker,
    proj: &str,
    lnd_node: &LndImage,
) -> Result<(LndRPC, Option<String>)> {
    let cert_path = "/home/.lnd/tls.cert";
    let cert = dl_cert(docker, &lnd_node.name, cert_path).await?;
    try_unlock_lnd(&cert, proj, lnd_node).await?;
    let macpath = format!(
        "/home/.lnd/data/chain/bitcoin/{}/admin.macaroon",
        lnd_node.network
    );
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
async fn try_dl(docker: &Docker, name: &str, path: &str) -> Result<Vec<u8>> {
    for _ in 0..60 {
        if let Ok(bytes) = download_from_container(docker, &domain(name), path).await {
            return Ok(bytes);
        }
        sleep_ms(500).await;
    }
    Err(anyhow!(format!(
        "try_dl failed to find {} in {}",
        path, name
    )))
}

// PEM encoded
pub async fn dl_cert(docker: &Docker, lnd_name: &str, path: &str) -> Result<String> {
    let cert_bytes = try_dl(docker, lnd_name, path).await?;
    Ok(String::from_utf8_lossy(&cert_bytes[..]).to_string())
}

// hex encoded
pub async fn dl_macaroon(docker: &Docker, lnd_name: &str, path: &str) -> Result<String> {
    let mac_bytes = try_dl(docker, lnd_name, path).await?;
    Ok(hex::encode(mac_bytes))
}

async fn try_unlock_lnd(cert: &str, proj: &str, lnd_node: &LndImage) -> Result<()> {
    let mut err = anyhow!("try_unlock_lnd never started");
    for _ in 0..60 {
        match unlock_lnd(cert, proj, lnd_node).await {
            Ok(_) => return Ok(()),
            Err(e) => err = e,
        }
        sleep_ms(500).await;
    }
    Err(anyhow!(format!("try_unlock_lnd failed: {:?}", err)))
}
pub async fn unlock_lnd(cert: &str, proj: &str, lnd_node: &LndImage) -> Result<()> {
    let secs = secrets::load_secrets(proj).await;
    // UNLOCK LND
    let unlock_port = lnd_node.http_port.clone().context("no unlock port")?;
    let unlocker = LndUnlocker::new(lnd_node, cert)
        .await
        .map_err(|e| anyhow!(format!("LndUnlocker::new failed: {}", e)))?;
    if let Some(_) = secs.get(&lnd_node.name) {
        let ur = unlocker.unlock_wallet(&lnd_node.unlock_password).await?;
        if let Some(err_msg) = ur.message {
            if !err_msg.contains("wallet already unlocked") {
                log::error!("FAILED TO UNLOCK LND {:?}", err_msg);
            }
        } else {
            log::info!("LND WALLET UNLOCKED!");
        }
    } else {
        let seed = unlocker.gen_seed().await?;
        if let Some(msg) = seed.message {
            log::error!("gen seed error: {}", msg);
        }
        let mnemonic = seed.cipher_seed_mnemonic.expect("no mnemonic");
        let ir = unlocker
            .init_wallet(&lnd_node.unlock_password, mnemonic.clone())
            .await?;
        if let Some(err_msg) = ir.message {
            log::error!("FAILED TO INIT LND {:?}", err_msg);
        } else {
            log::info!("LND WALLET INITIALIZED!");
        }
        secrets::add_to_secrets(proj, &lnd_node.name, &mnemonic.clone().join(" ")).await;
    };
    Ok(())
}

pub async fn relay_client(proj: &str, relay: &RelayImage) -> Result<RelayAPI> {
    let secs = secrets::load_secrets(proj).await;
    let relay_token = match secs.get(&relay.name) {
        Some(token) => token.clone(),
        None => secrets::random_word(12),
    };
    let api = RelayAPI::new(&relay, &relay_token, false).await?;
    let has_admin = api.try_has_admin().await?.response;
    if has_admin {
        log::info!("relay admin exists already");
        return Ok(api);
    }
    sleep_ms(400).await;
    let root_pubkey = api.initial_admin_pubkey().await?;
    let claim_res = api.claim_user(&root_pubkey, &relay_token).await?;
    secrets::add_to_secrets(proj, &relay.name, &relay_token).await;
    println!("Relay Root User claimed! {}", claim_res.response.id);
    Ok(api)
}

pub async fn sleep_ms(n: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(n)).await;
}
