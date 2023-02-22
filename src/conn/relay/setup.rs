use crate::conn::relay::RelayAPI;
use crate::images;
use crate::secrets;
use crate::utils::sleep_ms;
use anyhow::Result;
use images::relay::RelayImage;

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
