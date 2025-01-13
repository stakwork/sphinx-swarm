use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crate::{
    builder::find_img,
    cmd::{ChangeUserPasswordBySuperAdminInfo, GetDockerImageTagsDetails},
    config::{LightningPeer, Node, State},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSwarmBody {
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChangePasswordBySuperAdminResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwarmResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

pub async fn update_swarm() -> Result<String> {
    let password = std::env::var("SWARM_UPDATER_PASSWORD").unwrap_or(String::new());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build swarm updater reqwest client");

    let route = format!("http://172.17.0.1:3003/restart");

    let body = UpdateSwarmBody {
        password: password.to_string(),
    };
    let response = client.post(route.as_str()).json(&body).send().await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_image_tags(image_details: GetDockerImageTagsDetails) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build swarm updater reqwest client");

    let url = format!(
        "https://hub.docker.com/v2/repositories/{}/tags?page={}&page_size={}",
        image_details.org_image_name, image_details.page, image_details.page_size
    );

    let response = client.get(url.as_str()).send().await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn change_swarm_user_password_by_user_admin(
    state: &mut State,
    current_user_id: u32,
    password_change_details: ChangeUserPasswordBySuperAdminInfo,
    must_save_stack: &mut bool,
) -> ChangePasswordBySuperAdminResponse {
    if password_change_details.username == "super" {
        return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "Unauthorized, you are not authorized to change Super admin password"
                .to_string(),
        };
    }

    // check if super admin is performing his operation
    let superadmin_details = state
        .stack
        .users
        .iter()
        .find(|user| user.id == current_user_id);

    if superadmin_details.is_none() {
        return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "invalid super admin details".to_string(),
        };
    }

    if superadmin_details.unwrap().username != "super" {
        return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "Unauthorized, you don't have permissions to perform this operation"
                .to_string(),
        };
    }

    // we should find a way to find users properly and not reply on username, user_id might be a great option
    match state
        .stack
        .users
        .iter()
        .position(|u| u.username == password_change_details.username)
    {
        Some(pos) => {
            let old_pass_hash = &state.stack.users[pos].pass_hash;
            let verify_password =
                match bcrypt::verify(password_change_details.current_password, old_pass_hash) {
                    Ok(result) => result,
                    Err(err) => {
                        return ChangePasswordBySuperAdminResponse {
                            success: false,
                            message: err.to_string(),
                        }
                    }
                };
            if verify_password {
                let new_password_hash = match bcrypt::hash(
                    password_change_details.new_password,
                    bcrypt::DEFAULT_COST,
                ) {
                    Ok(hash) => hash,
                    Err(err) => {
                        return ChangePasswordBySuperAdminResponse {
                            success: false,
                            message: err.to_string(),
                        }
                    }
                };
                state.stack.users[pos].pass_hash = new_password_hash;
                *must_save_stack = true;

                return ChangePasswordBySuperAdminResponse {
                    success: true,
                    message: "password updated successfully".to_string(),
                };
            } else {
                return ChangePasswordBySuperAdminResponse {
                    success: false,
                    message: "You provided invalid password".to_string(),
                };
            }
        }
        None => {
            return ChangePasswordBySuperAdminResponse {
                success: false,
                message: "Invalid user".to_string(),
            }
        }
    }
}

pub fn add_new_lightning_peer(
    state: &mut State,
    info: LightningPeer,
    must_save_stack: &mut bool,
) -> SwarmResponse {
    if info.pubkey.is_empty() || info.alias.is_empty() {
        return SwarmResponse {
            success: false,
            message: "pubkey and alias cannot be empty".to_string(),
            data: None,
        };
    }

    let lightning_peers_clone = state.stack.lightning_peers.clone();

    if let Some(mut lightning_peers) = lightning_peers_clone {
        let peer_exist = lightning_peers
            .iter()
            .position(|peer| peer.pubkey == info.pubkey);
        if peer_exist.is_some() {
            return SwarmResponse {
                success: false,
                message: "public key already exist, please update".to_string(),
                data: None,
            };
        }
        lightning_peers.push(info);
        state.stack.lightning_peers = Some(lightning_peers);
    } else {
        state.stack.lightning_peers = Some(vec![info]);
    }

    *must_save_stack = true;
    SwarmResponse {
        success: true,
        message: "peer added successfully".to_string(),
        data: None,
    }
}

pub fn update_lightning_peer(
    state: &mut State,
    info: LightningPeer,
    must_save_stack: &mut bool,
) -> SwarmResponse {
    if info.alias.is_empty() {
        return SwarmResponse {
            success: false,
            message: "alias cannot be empty".to_string(),
            data: None,
        };
    }

    if state.stack.lightning_peers.is_none() {
        return SwarmResponse {
            success: false,
            message: "pubkey does not exist".to_string(),
            data: None,
        };
    };

    if let Some(mut clone_lightning_peers) = state.stack.lightning_peers.clone() {
        let pos = clone_lightning_peers
            .iter()
            .position(|peer| peer.pubkey == info.pubkey);

        if pos.is_none() {
            return SwarmResponse {
                success: false,
                message: "invalid pubkey".to_string(),
                data: None,
            };
        }

        clone_lightning_peers[pos.unwrap()] = info;
        state.stack.lightning_peers = Some(clone_lightning_peers);
    }
    *must_save_stack = true;
    SwarmResponse {
        success: true,
        message: "alias updated successfully".to_string(),
        data: None,
    }
}

pub fn get_neo4j_password(nodes: &Vec<Node>) -> SwarmResponse {
    let neo4j = find_img("neo4j", nodes);
    match neo4j {
        Ok(image) => match image.as_neo4j() {
            Ok(neo4j) => SwarmResponse {
                success: true,
                message: "neo4j password suucessfully retrived".to_string(),
                data: Some(serde_json::Value::String(neo4j.password)),
            },
            Err(err) => {
                log::error!("Error getting neo4j image: {}", err.to_string());
                SwarmResponse {
                    success: false,
                    message: err.to_string(),
                    data: None,
                }
            }
        },
        Err(err) => SwarmResponse {
            success: false,
            message: err.to_string(),
            data: None,
        },
    }
}
