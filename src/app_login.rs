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
    let detail = details
        .get_mut(challenge)
        .ok_or(anyhow::anyhow!("challenge doesn't exist"))?;
    // verify token first
    let unsigned = token::Token::from_base64(token)?;
    let pubkey = unsigned.recover()?;
    let state = config::STATE.lock().await;

    match state
        .stack
        .users
        .iter()
        .find(|u| u.pubkey == Some(pubkey.to_string()))
    {
        Some(_user) => {
            *detail = pubkey.to_string();
            Ok(VerifyResponse {
                success: true,
                message: "Successfully verified token".to_string(),
            })
        }
        None => Ok(VerifyResponse {
            success: false,
            message: "invalid token".to_string(),
        }),
    }
}

pub async fn check_challenge_status(challenge: &str) -> Result<ChallengeStatus> {
    let details = DETAILS.lock().await;
    let pubkey = details
        .get(challenge)
        .ok_or(anyhow::anyhow!("challenge doesn't exist"))?;

    let state = config::STATE.lock().await;

    match state
        .stack
        .users
        .iter()
        .find(|u| u.pubkey == Some(pubkey.to_string()))
    {
        Some(user) => Ok(ChallengeStatus {
            success: true,
            token: auth::make_jwt(user.id)?,
        }),
        None => Ok(ChallengeStatus {
            success: false,
            token: "".to_string(),
        }),
    }
}
