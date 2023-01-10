use crate::conn::lnd::{lndrpc::LndRPC, unlocker::LndUnlocker};
use crate::conn::relay::RelayAPI;
use crate::images;
use crate::secrets;
use anyhow::{Context, Result};
use images::LndImage;
use rocket::tokio;

pub async fn lnd_clients(
    proj: &str,
    lnd_node: &LndImage,
    secs: &secrets::Secrets,
    name: &str,
) -> Result<(LndRPC, Option<String>)> {
    // returns address if test mine needed
    unlock_lnd(proj, lnd_node, secs, name).await?;
    sleep(1).await;
    let mut client = LndRPC::new(proj, lnd_node).await?;
    let bal = client.get_balance().await?;
    if bal.confirmed_balance > 0 {
        return Ok((client, None));
    }
    let addy = client.new_address().await?;
    Ok((client, Some(addy.address)))
}

pub async fn unlock_lnd(
    proj: &str,
    lnd_node: &LndImage,
    secs: &secrets::Secrets,
    name: &str,
) -> Result<()> {
    // UNLOCK LND
    let cert_path = format!("vol/{}/{}/tls.cert", proj, name);
    let unlock_port = lnd_node.http_port.clone().context("no unlock port")?;
    let unlocker = LndUnlocker::new(&unlock_port, &cert_path).await?;
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

async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
