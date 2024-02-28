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

pub static DETAILS: Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn generate_challenge() -> String {
    let challenge = secrets::random_word(16);
    let mut details = DETAILS.lock().await;
    details.insert(challenge.clone(), false);
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

    match state.stack.users.iter().find(|u| u.pubkey == Some(pubkey)) {
        Some(_user) => {
            *detail = true;
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
