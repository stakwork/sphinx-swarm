use crate::auth;
use crate::config;
use anyhow::Result;
use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use serde::Deserialize;
use serde::Serialize;
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
    pub message: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct GetSignupChallengeResponse {
    pub success: bool,
    pub pubkey: String,
    pub message: String,
}

pub static DETAILS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub static SIGNUP_DETAILS: Lazy<Mutex<HashMap<String, (u32, Option<String>)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn generate_challenge() -> String {
    let challenge = secrets::random_word(16);
    let mut details = DETAILS.lock().await;
    details.insert(challenge.clone(), "".to_string());
    challenge.to_string()
}

pub async fn generate_signup_challenge(user_id: u32) -> String {
    let challenge = secrets::random_word(16);
    let mut details = SIGNUP_DETAILS.lock().await;
    details.insert(challenge.clone(), (user_id, None));
    challenge.to_string()
}

pub async fn verify_signed_token(challenge: &str, token: &str) -> Result<VerifyResponse> {
    let mut signup_details = SIGNUP_DETAILS.lock().await;

    match signup_details.get_mut(challenge) {
        Some(signup_detail) => {
            //get the key
            let (user_id, _value) = signup_detail;
            //decrypt token
            let unsigned = token::Token::from_base64(token)?;
            let pubkey = unsigned.recover()?;

            *signup_detail = (*user_id, Some(pubkey.to_string()));
            return Ok(VerifyResponse {
                success: true,
                message: "Successfully verified token".to_string(),
            });
        }
        None => drop(signup_details),
    }
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
    let mut details = DETAILS.lock().await;
    let detail = details
        .get_mut(challenge)
        .ok_or(anyhow::anyhow!("challenge doesn't exist"))?;

    if res.success {
        *detail = pubkey.to_string();
    } else {
        *detail = "unauthorize".to_string()
    }

    Ok(res)
}

pub async fn check_challenge_status(challenge: &str) -> Result<ChallengeStatus> {
    let mut details = DETAILS.lock().await;
    let pubkey = details
        .get(challenge)
        .ok_or(anyhow::anyhow!("challenge doesn't exist"))?
        .clone();

    if pubkey == "unauthorize" {
        details.remove(challenge);
        return Ok(ChallengeStatus {
            success: false,
            token: "".to_string(),
            message: "unauthorized".to_string(),
        });
    }
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
            message: "login successfully".to_string(),
        },
        None => ChallengeStatus {
            success: false,
            token: "".to_string(),
            message: "waiting for token".to_string(),
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

pub async fn find_challenge_from_signup_hashmap(challenge: &str) -> Option<(u32, Option<String>)> {
    let details = SIGNUP_DETAILS.lock().await;
    let detail = details.get(challenge)?;
    Some(detail.clone())
}
pub async fn remove_signup_challenge(challenge: &str) -> Option<(u32, Option<String>)> {
    let mut details = SIGNUP_DETAILS.lock().await;
    details.remove(challenge)
}

pub async fn sign_up_admin_pubkey(
    body: crate::cmd::SignUpAdminPubkeyDetails,
    must_save_stack: &mut bool,
    state: &mut crate::config::State,
) -> Result<GetSignupChallengeResponse> {
    let res = match find_challenge_from_signup_hashmap(&body.challenge).await {
        Some(user_detail) => {
            // check user id matches
            if body.user_id != user_detail.0 {
                return Ok(GetSignupChallengeResponse {
                    success: false,
                    pubkey: "".to_string(),
                    message: "you are not unauthorized to access this challenge".to_string(),
                });
            }
            let pubkey = user_detail.1;
            // check if verified
            if pubkey.is_none() {
                return Ok(GetSignupChallengeResponse {
                    success: false,
                    pubkey: "".to_string(),
                    message: "not yet verified".to_string(),
                });
            }
            // safe to unwrap here since "is_none" was checked above
            let pubkey = pubkey.unwrap();
            match state.stack.users.iter().position(|u| u.id == body.user_id) {
                Some(ui) => {
                    state.stack.users[ui].pubkey = Some(pubkey.clone());
                    *must_save_stack = true;
                    let boltwall = crate::handler::find_boltwall(&state.stack.nodes)?;
                    crate::conn::boltwall::add_admin_pubkey(&boltwall, &pubkey, &"".to_string())
                        .await?;
                    remove_signup_challenge(&body.challenge).await;
                    GetSignupChallengeResponse {
                        success: true,
                        pubkey: pubkey.to_string(),
                        message: "signup successful".to_string(),
                    }
                }
                None => GetSignupChallengeResponse {
                    success: false,
                    pubkey: "".to_string(),
                    message: "you are not unauthorized to access this challenge".to_string(),
                },
            }
        }
        None => GetSignupChallengeResponse {
            success: false,
            pubkey: "".to_string(),
            message: "challenge not found".to_string(),
        },
    };
    Ok(res)
}
