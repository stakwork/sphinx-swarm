use crate::{dock, images::lnd::LndImage, secrets, utils::sleep_ms};
use anyhow::{anyhow, Result};
use bollard::Docker;

use super::unlocker::LndUnlocker;

pub fn strip_pem_prefix_suffix(s: &str) -> String {
    let mut ret = s.to_string();
    ret.retain(|c| !c.is_whitespace());
    if let Some(no_prefix) = ret.strip_prefix("-----BEGINCERTIFICATE-----") {
        ret = no_prefix.to_string();
    }
    if let Some(no_suffix) = ret.strip_suffix("-----ENDCERTIFICATE-----") {
        ret = no_suffix.to_string();
    }
    ret
}

// PEM encoded (with -----BEGIN CERTIFICATE----- and -----END CERTIFICATE-----)
pub async fn dl_cert(docker: &Docker, lnd_name: &str, path: &str) -> Result<String> {
    let cert_bytes = dock::try_dl(docker, lnd_name, path).await?;
    Ok(String::from_utf8_lossy(&cert_bytes[..]).to_string())
}

pub async fn dl_cert_to_base64(docker: &Docker, lnd_name: &str, path: &str) -> Result<String> {
    let cert_bytes = dock::try_dl(docker, lnd_name, path).await?;
    Ok(base64::encode(cert_bytes))
}

// hex encoded
pub async fn dl_macaroon(docker: &Docker, lnd_name: &str, path: &str) -> Result<String> {
    let mac_bytes = dock::try_dl(docker, lnd_name, path).await?;
    Ok(hex::encode(mac_bytes))
}

pub async fn try_unlock_lnd(cert: &str, proj: &str, lnd_node: &LndImage) -> Result<()> {
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
