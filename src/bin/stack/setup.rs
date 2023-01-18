use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use images::lnd::LndImage;
use rocket::tokio;
use sphinx_swarm::config::Clients;
use sphinx_swarm::conn::lnd::{lndrpc::LndRPC, unlocker::LndUnlocker};
use sphinx_swarm::conn::relay::RelayAPI;
use sphinx_swarm::dock::download_from_container;
use sphinx_swarm::images;
use sphinx_swarm::secrets;
use sphinx_swarm::utils::domain;

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

// returns LndRPC client and address if test mine needed
pub async fn lnd_clients(
    docker: &Docker,
    proj: &str,
    lnd_node: &LndImage,
    secs: &secrets::Secrets,
) -> Result<(LndRPC, Option<String>)> {
    sleep(5).await;
    let cert = dl_cert(docker, &lnd_node.name, "/home/.lnd/tls.cert").await?;
    sleep(5).await;
    unlock_lnd(&cert, proj, lnd_node, secs).await?;
    sleep(5).await;
    let macpath = format!(
        "/home/.lnd/data/chain/bitcoin/{}/admin.macaroon",
        lnd_node.network
    );
    let mac = dl_macaroon(docker, &lnd_node.name, &macpath).await?;
    let mut client = LndRPC::new(proj, lnd_node, &cert, &mac)
        .await
        .map_err(|e| anyhow!(format!("LndRPC::new failed: {}", e)))?;
    let bal = client.get_balance().await?;
    if bal.confirmed_balance > 0 {
        return Ok((client, None));
    }
    let addy = client.new_address().await?;
    Ok((client, Some(addy.address)))
}

// PEM encoded
pub async fn dl_cert(docker: &Docker, lnd_name: &str, path: &str) -> Result<String> {
    let cert_bytes = download_from_container(docker, &domain(lnd_name), path).await?;
    Ok(String::from_utf8_lossy(&cert_bytes[..]).to_string())
}

// hex encoded
pub async fn dl_macaroon(docker: &Docker, lnd_name: &str, path: &str) -> Result<String> {
    let mac_bytes = download_from_container(docker, &domain(lnd_name), path).await?;
    Ok(hex::encode(mac_bytes))
}

pub async fn unlock_lnd(
    cert: &str,
    proj: &str,
    lnd_node: &LndImage,
    secs: &secrets::Secrets,
) -> Result<()> {
    // UNLOCK LND
    let unlock_port = lnd_node.http_port.clone().context("no unlock port")?;
    let unlocker = LndUnlocker::new(&unlock_port, cert)
        .await
        .map_err(|e| anyhow!(format!("LndUnlocker::new failed: {}", e)))?;
    if let Some(_) = secs.get(&lnd_node.name) {
        let ur = unlocker.unlock_wallet(&lnd_node.unlock_password).await?;
        if let Some(err_msg) = ur.message {
            log::error!("FAILED TO UNLOCK LND {:?}", err_msg);
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

pub async fn relay_root_user(proj: &str, name: &str, api: RelayAPI) -> Result<RelayAPI> {
    let has_admin = api.has_admin().await?;
    if has_admin {
        log::info!("relay admin exists already");
        return Ok(api);
    }
    let new_user = api.add_user().await?;
    let token = secrets::random_word(12);
    let _id = api.claim_user(&new_user.public_key, &token).await?;
    secrets::add_to_secrets(proj, name, &token).await;
    Ok(api)
}

pub async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
