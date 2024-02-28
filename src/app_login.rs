use crate::auth;
use crate::config;
use anyhow::Result;
use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use std::collections::HashMap;

use crate::secrets;
use sphinx_auther::token;

#[derive(Debug)]
pub struct VerifyResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug)]
pub struct ChallengeStatus {
    pub success: bool,
    pub token: String,
}

pub static DETAILS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn generate_challenge() -> String {
    let challenge = secrets::random_word(16);
    let mut details = DETAILS.lock().await;
    details.insert(challenge.clone(), "".to_string());
    challenge.to_string()
}

pub async fn verify_signed_token(challenge: &str, token: &str) -> Result<VerifyResponse> {
    let mut details = DETAILS.lock().await;
    let _detail = details
        .get_mut(challenge)
        .ok_or(anyhow::anyhow!("challenge doesn't exist"))?;
    drop(details);

    // verify token first
    let unsigned = token::Token::from_base64(token)?;
    let pubkey = unsigned.recover()?;
    let state = config::STATE.lock().await;
    let res = match state
        .stack
        .users
        .iter()
        .find(|u| u.pubkey == Some(pubkey.to_string()))
    {
        Some(_user) => VerifyResponse {
            success: true,
            message: "Successfully verified token".to_string(),
        },
        None => VerifyResponse {
            success: false,
            message: "invalid token".to_string(),
        },
    };
    drop(state);

    if res.success {
        let mut details = DETAILS.lock().await;
        let detail = details
            .get_mut(challenge)
            .ok_or(anyhow::anyhow!("challenge doesn't exist"))?;
        *detail = pubkey.to_string();
    }

    Ok(res)
}

pub async fn check_challenge_status(challenge: &str) -> Result<ChallengeStatus> {
    let details = DETAILS.lock().await;
    let pubkey = details
        .get(challenge)
        .ok_or(anyhow::anyhow!("challenge doesn't exist"))?
        .clone();
    drop(details);

    let state = config::STATE.lock().await;
    let res = match state
        .stack
        .users
        .iter()
        .find(|u| u.pubkey == Some(pubkey.to_string()))
    {
        Some(user) => ChallengeStatus {
            success: true,
            token: auth::make_jwt(user.id)?,
        },
        None => ChallengeStatus {
            success: false,
            token: "".to_string(),
        },
    };
    drop(state);

    //remove successfully verified challenge from hashmap
    if res.success {
        let mut details = DETAILS.lock().await;
        details.remove(challenge);
    }

    Ok(res)
}
