use anyhow::Result;
use bollard::Docker;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, time::Duration};

use crate::{
    builder::find_img,
    cmd::{
        AssignSwarmNewDetails, ChangeUserPasswordBySuperAdminInfo, GetDockerImageTagsDetails,
        UpdateEnvRequest,
    },
    config::{LightningPeer, Node, State},
    dock::stop_and_remove,
    images::{
        boltwall::{ExternalLnd, LndCreds},
        Image,
    },
    utils::{domain, is_using_port_based_ssl, update_or_write_to_env_file},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSwarmBody {
    pub password: String,
    pub port_based_ssl: bool,
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
        port_based_ssl: is_using_port_based_ssl(),
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
    user_id: Option<u32>,
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

    if user_id.is_none() {
        return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "invalid user".to_string(),
        };
    }
    let current_user_id = user_id.unwrap();

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

pub async fn update_env_variables(
    docker: &Docker,
    update_value: &mut UpdateEnvRequest,
    state: &mut State,
    must_save_stack: &mut bool,
) -> SwarmResponse {
    let mut error_messages: Vec<String> = Vec::new();
    // write to stack.yml file
    if update_value.id.is_some() && update_value.clone().id.unwrap() == "boltwall" {
        update_boltwall_env(state, &mut update_value.values);
    }

    log::info!(
        "Updating env variables for {:?}: {:?}",
        update_value.id,
        update_value.values
    );

    // write to .env file
    if let Err(e) = update_or_write_to_env_file(&update_value.values) {
        return SwarmResponse {
            success: false,
            message: e.to_string(),
            data: None,
        };
    }

    *must_save_stack = true;
    // stop the expected service(Boltwall and Jarvis)

    if let Some(host) = update_value.values.get("HOST") {
        for node in state.stack.nodes.iter_mut() {
            if let Node::Internal(img) = node {
                img.set_host(host);
            }
        }
    };
    for node in state.stack.nodes.iter_mut() {
        match stop_and_remove(docker, &domain(&node.name())).await {
            Ok(_) => log::info!("{} stopped and removed", node.name()),
            Err(e) => {
                error_messages.push(format!("could not stop {}: {}", node.name(), e.to_string()));
            }
        }
    }

    if error_messages.len() > 0 {
        return SwarmResponse {
            success: false,
            message: error_messages.join(", "),
            data: None,
        };
    }

    SwarmResponse {
        success: true,
        message: "Environment variables updated successfully".to_string(),
        data: None,
    }
}

fn get_boltwall_stored_env() -> HashMap<String, (String, String)> {
    let mut boltwall_envs: HashMap<String, (String, String)> = HashMap::new();
    boltwall_envs.insert(
        "SESSION_SECRET".to_string(),
        ("session_secret".to_string(), "".to_string()),
    );

    boltwall_envs.insert(
        "LND_SOCKET".to_string(),
        ("address".to_string(), "EXTERNAL_LND_ADDRESS".to_string()),
    );

    boltwall_envs.insert(
        "LND_TLS_CERT".to_string(),
        ("cert".to_string(), "EXTERNAL_LND_CERT".to_string()),
    );

    boltwall_envs.insert(
        "LND_MACAROON".to_string(),
        ("macaroon".to_string(), "EXTERNAL_LND_MACAROON".to_string()),
    );

    boltwall_envs.insert(
        "ADMIN_TOKEN".to_string(),
        ("admin_token".to_string(), "".to_string()),
    );

    boltwall_envs.insert(
        "STAKWORK_SECRET".to_string(),
        ("stakwork_secret".to_string(), "".to_string()),
    );

    boltwall_envs.insert(
        "REQUEST_PER_SECONDS".to_string(),
        ("request_per_seconds".to_string(), "".to_string()),
    );

    boltwall_envs.insert(
        "MAX_REQUEST_SIZE".to_string(),
        ("max_request_limit".to_string(), "".to_string()),
    );

    return boltwall_envs;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyToBeUpdated {
    pub old_key: String,
    pub new_key: String,
}

fn update_boltwall_env(state: &mut State, env_values: &mut HashMap<String, String>) {
    let mut to_be_updated_env: Vec<KeyToBeUpdated> = Vec::new();
    let nodes: Vec<Node> = state
        .stack
        .nodes
        .iter()
        .map(|n| match n {
            Node::External(e) => Node::External(e.clone()),
            Node::Internal(i) => match i.clone() {
                Image::BoltWall(mut b) => {
                    let boltwall_envs = get_boltwall_stored_env();
                    for (key, value) in &mut *env_values {
                        if let Some(boltwall_values) = boltwall_envs.get(key) {
                            if !boltwall_values.1.is_empty() {
                                to_be_updated_env.push(KeyToBeUpdated {
                                    new_key: boltwall_values.1.clone(),
                                    old_key: key.clone(),
                                });
                            }
                            if boltwall_values.0 == "session_secret" {
                                b.session_secret = value.clone();
                            } else if boltwall_values.0 == "address" {
                                if let Some(external_lnd) = b.external_lnd {
                                    b.external_lnd = Some(ExternalLnd {
                                        address: value.clone(),
                                        creds: LndCreds {
                                            cert: external_lnd.creds.cert,
                                            macaroon: external_lnd.creds.macaroon,
                                        },
                                    });
                                }
                            } else if boltwall_values.0 == "cert" {
                                if let Some(external_lnd) = b.external_lnd {
                                    b.external_lnd = Some(ExternalLnd {
                                        address: external_lnd.address,
                                        creds: LndCreds {
                                            cert: value.clone(),
                                            macaroon: external_lnd.creds.macaroon,
                                        },
                                    });
                                }
                            } else if boltwall_values.0 == "macaroon" {
                                if let Some(external_lnd) = b.external_lnd {
                                    b.external_lnd = Some(ExternalLnd {
                                        address: external_lnd.address,
                                        creds: LndCreds {
                                            cert: external_lnd.creds.cert,
                                            macaroon: value.clone(),
                                        },
                                    });
                                }
                            } else if boltwall_values.0 == "admin_token" {
                                b.set_admin_token(&value.clone());
                            } else if boltwall_values.0 == "stakwork_secret" {
                                b.set_stakwork_token(&value.clone());
                            } else if boltwall_values.0 == "request_per_seconds" {
                                if let Ok(parse_value) = value.parse() {
                                    b.set_request_per_seconds(parse_value);
                                }
                            } else if boltwall_values.0 == "max_request_limit" {
                                b.set_max_request_limit(&value.clone());
                            }
                        }
                    }
                    Node::Internal(Image::BoltWall(b))
                }
                _ => Node::Internal(i.clone()),
            },
        })
        .collect();

    state.stack.nodes = nodes;

    for env in to_be_updated_env {
        if let Some(value) = env_values.get(&env.old_key) {
            env_values.insert(env.new_key, value.clone());
            env_values.remove(&env.old_key);
        }
    }
}

pub async fn handle_assign_reserved_swarm_to_active(
    docker: &Docker,
    new_details: &AssignSwarmNewDetails,
    user_id: Option<u32>,
    state: &mut State,
    must_save_stack: &mut bool,
) -> SwarmResponse {
    let mut error_messages: Vec<String> = Vec::new();

    if new_details.new_password.is_some() && new_details.old_password.is_some() {
        let change_password_response = change_swarm_user_password_by_user_admin(
            state,
            user_id,
            ChangeUserPasswordBySuperAdminInfo {
                new_password: new_details.new_password.clone().unwrap(),
                current_password: new_details.old_password.clone().unwrap(),
                username: "admin".to_string(),
            },
            must_save_stack,
        )
        .await;
        log::info!("Change password response: {:?}", change_password_response);
        if !change_password_response.success {
            error_messages.push(change_password_response.message);
        }
    }

    if new_details.env.is_some() {
        let envs = new_details.env.clone().unwrap();
        let update_env_response = update_env_variables(
            docker,
            &mut UpdateEnvRequest {
                id: Some("boltwall".to_string()),
                values: envs,
            },
            state,
            must_save_stack,
        )
        .await;
        log::info!("Update env response: {:?}", update_env_response);
        if !update_env_response.success {
            error_messages.push(update_env_response.message);
        }
    }
    *must_save_stack = true;

    if error_messages.len() > 0 {
        return SwarmResponse {
            success: false,
            message: error_messages.join(", "),
            data: None,
        };
    }

    SwarmResponse {
        success: true,
        message: "New Swarm details assigned successfully".to_string(),
        data: None,
    }
}
