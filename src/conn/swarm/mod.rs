use anyhow::Result;
use bollard::Docker;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, time::Duration};

use crate::{
    builder::find_img,
    cmd::{
        AssignSwarmNewDetails, BotBalanceRes, ChangeUserPasswordBySuperAdminInfo,
        GetDockerImageTagsDetails, UpdateEnvRequest,
    },
    config,
    config::{LightningPeer, Node, Stack},
    conn::boltwall::add_admin_pubkey,
    dock::{restart_node_container_global, stop_and_remove},
    images::{
        boltwall::{BoltwallImage, ExternalLnd, LndCreds},
        Image,
    },
    utils::{
        docker_domain, domain, getenv, is_using_port_based_ssl, make_reqwest_client,
        update_or_write_to_env_file,
    },
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSwarmBody {
    pub password: String,
    pub port_based_ssl: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSslCertSwarmBody {
    pub password: String,
    pub cert_bucket_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwarmRestarterRes {
    pub ok: Option<bool>,
    pub error: Option<String>,
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
        .user_agent("sphinx-swarm/1.0")
        .timeout(Duration::from_secs(40))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build get image tags reqwest client");

    let mut url = format!(
        "https://hub.docker.com/v2/repositories/{}/tags?page={}&page_size={}",
        image_details.org_image_name, image_details.page, image_details.page_size
    );

    if image_details.host.is_some() && image_details.host.unwrap() == "Github" {
        let image_name_parts: Vec<&str> = image_details.org_image_name.split("/").collect();
        if image_name_parts.len() != 2 {
            log::error!(
                "Invalid image name format: {}",
                image_details.org_image_name
            );
            return Ok("Invalid image name format".to_string());
        }
        let github_container_name = image_name_parts[1];
        let org_name = image_name_parts[0];
        url = format!(
            "https://api.github.com/orgs/{}/packages/container/{}/versions",
            org_name, github_container_name,
        );

        let token = getenv("GITHUB_PAT").unwrap_or("".to_string());

        let response = client
            .get(url.clone())
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let response_text = response.text().await?;

        return Ok(response_text);
    }

    let response = client.get(url.as_str()).send().await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn change_swarm_user_password_by_user_admin(
    proj: &str,
    user_id: Option<u32>,
    password_change_details: ChangeUserPasswordBySuperAdminInfo,
) -> ChangePasswordBySuperAdminResponse {
    if password_change_details.username == "super" {
        return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "Unauthorized, you are not authorized to change Super admin password"
                .to_string(),
        };
    }

    let current_user_id = match user_id {
        Some(id) => id,
        None => return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "invalid user".to_string(),
        },
    };

    // 1. Read what we need (brief read lock)
    let user_data = config::stack_read(|s| {
        let is_super = s.users.iter()
            .find(|u| u.id == current_user_id)
            .map(|u| u.username == "super")
            .unwrap_or(false);
        let target_hash = s.users.iter()
            .find(|u| u.username == password_change_details.username)
            .map(|u| u.pass_hash.clone());
        (is_super, target_hash)
    }).await;

    if !user_data.0 {
        return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "Unauthorized, you don't have permissions to perform this operation"
                .to_string(),
        };
    }

    let old_pass_hash = match user_data.1 {
        Some(h) => h,
        None => return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "Invalid user".to_string(),
        },
    };

    // 2. bcrypt verify + hash (CPU-heavy, no lock held)
    let verify_password = match bcrypt::verify(password_change_details.current_password, &old_pass_hash) {
        Ok(result) => result,
        Err(err) => return ChangePasswordBySuperAdminResponse {
            success: false,
            message: err.to_string(),
        },
    };

    if !verify_password {
        return ChangePasswordBySuperAdminResponse {
            success: false,
            message: "You provided invalid password".to_string(),
        };
    }

    let new_password_hash = match bcrypt::hash(
        password_change_details.new_password,
        bcrypt::DEFAULT_COST,
    ) {
        Ok(hash) => hash,
        Err(err) => return ChangePasswordBySuperAdminResponse {
            success: false,
            message: err.to_string(),
        },
    };

    // 3. Persist (brief write lock + save)
    let username = password_change_details.username.clone();
    config::stack_write(proj, |s| {
        if let Some(u) = s.users.iter_mut().find(|u| u.username == username) {
            u.pass_hash = new_password_hash;
        }
    }).await;

    ChangePasswordBySuperAdminResponse {
        success: true,
        message: "password updated successfully".to_string(),
    }
}

pub fn add_new_lightning_peer(
    stack: &mut Stack,
    info: LightningPeer,
) -> SwarmResponse {
    if info.pubkey.is_empty() || info.alias.is_empty() {
        return SwarmResponse {
            success: false,
            message: "pubkey and alias cannot be empty".to_string(),
            data: None,
        };
    }

    let lightning_peers_clone = stack.lightning_peers.clone();

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
        stack.lightning_peers = Some(lightning_peers);
    } else {
        stack.lightning_peers = Some(vec![info]);
    }

    SwarmResponse {
        success: true,
        message: "peer added successfully".to_string(),
        data: None,
    }
}

pub fn update_lightning_peer(
    stack: &mut Stack,
    info: LightningPeer,
) -> SwarmResponse {
    if info.alias.is_empty() {
        return SwarmResponse {
            success: false,
            message: "alias cannot be empty".to_string(),
            data: None,
        };
    }

    if stack.lightning_peers.is_none() {
        return SwarmResponse {
            success: false,
            message: "pubkey does not exist".to_string(),
            data: None,
        };
    };

    if let Some(mut clone_lightning_peers) = stack.lightning_peers.clone() {
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
        stack.lightning_peers = Some(clone_lightning_peers);
    }
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

fn find_boltwall_opt(nodes: &Vec<Node>) -> Option<crate::images::boltwall::BoltwallImage> {
    nodes
        .iter()
        .find_map(|n| n.as_internal().ok().and_then(|i| i.as_boltwall().ok()))
}

pub fn get_bot_token(nodes: &Vec<Node>) -> SwarmResponse {
    let bot = find_img("bot", nodes);
    match bot {
        Ok(image) => match image.as_bot() {
            Ok(bot) => {
                let boltwall = find_boltwall_opt(nodes);
                SwarmResponse {
                    success: true,
                    message: "bot admin token successfully retrieved".to_string(),
                    data: Some(serde_json::Value::String(bot.actual_admin_token(&boltwall))),
                }
            }
            Err(err) => SwarmResponse {
                success: false,
                message: err.to_string(),
                data: None,
            },
        },
        Err(err) => SwarmResponse {
            success: false,
            message: err.to_string(),
            data: None,
        },
    }
}

pub async fn get_bot_balance(nodes: &Vec<Node>) -> SwarmResponse {
    let bot = match find_img("bot", nodes).and_then(|i| i.as_bot()) {
        Ok(b) => b,
        Err(e) => {
            return SwarmResponse {
                success: false,
                message: e.to_string(),
                data: None,
            }
        }
    };
    let boltwall = find_boltwall_opt(nodes);
    let admin_token = bot.actual_admin_token(&boltwall);
    let url = format!("http://{}:{}/balance", docker_domain(&bot.name), bot.port);
    let client = make_reqwest_client();
    match client
        .get(&url)
        .header("x-admin-token", &admin_token)
        .send()
        .await
    {
        Ok(res) => match res.json::<BotBalanceRes>().await {
            Ok(bal) => SwarmResponse {
                success: true,
                message: "bot balance retrieved".to_string(),
                data: Some(serde_json::to_value(bal).unwrap_or(Value::Null)),
            },
            Err(e) => SwarmResponse {
                success: false,
                message: e.to_string(),
                data: None,
            },
        },
        Err(e) => SwarmResponse {
            success: false,
            message: e.to_string(),
            data: None,
        },
    }
}

pub async fn get_bot_payments(nodes: &Vec<Node>) -> SwarmResponse {
    let bot = match find_img("bot", nodes).and_then(|i| i.as_bot()) {
        Ok(b) => b,
        Err(e) => {
            return SwarmResponse {
                success: false,
                message: e.to_string(),
                data: None,
            }
        }
    };
    let boltwall = find_boltwall_opt(nodes);
    let admin_token = bot.actual_admin_token(&boltwall);
    let url = format!("http://{}:{}/payments", docker_domain(&bot.name), bot.port);
    let client = make_reqwest_client();
    match client
        .get(&url)
        .header("x-admin-token", &admin_token)
        .send()
        .await
    {
        Ok(res) => match res.json::<Value>().await {
            Ok(data) => SwarmResponse {
                success: true,
                message: "bot payments retrieved".to_string(),
                data: Some(data),
            },
            Err(e) => SwarmResponse {
                success: false,
                message: e.to_string(),
                data: None,
            },
        },
        Err(e) => SwarmResponse {
            success: false,
            message: e.to_string(),
            data: None,
        },
    }
}

pub async fn create_bot_invoice(nodes: &Vec<Node>, amt_msat: u64) -> SwarmResponse {
    let bot = match find_img("bot", nodes).and_then(|i| i.as_bot()) {
        Ok(b) => b,
        Err(e) => {
            return SwarmResponse {
                success: false,
                message: e.to_string(),
                data: None,
            }
        }
    };
    let boltwall = find_boltwall_opt(nodes);
    let admin_token = bot.actual_admin_token(&boltwall);
    let url = format!("http://{}:{}/invoice", docker_domain(&bot.name), bot.port);
    let client = make_reqwest_client();
    let body = serde_json::json!({ "amt_msat": amt_msat });
    match client
        .post(&url)
        .header("x-admin-token", &admin_token)
        .json(&body)
        .send()
        .await
    {
        Ok(res) => match res.json::<Value>().await {
            Ok(data) => SwarmResponse {
                success: true,
                message: "invoice created".to_string(),
                data: Some(data),
            },
            Err(e) => SwarmResponse {
                success: false,
                message: e.to_string(),
                data: None,
            },
        },
        Err(e) => SwarmResponse {
            success: false,
            message: e.to_string(),
            data: None,
        },
    }
}

pub async fn update_env_variables(
    proj: &str,
    docker: &Docker,
    update_value: &mut UpdateEnvRequest,
) -> SwarmResponse {
    log::info!(
        "Updating env variables for {:?}: {:?}",
        update_value.id,
        update_value.values
    );

    // 1. Write to .env file (no lock needed)
    if let Err(e) = update_or_write_to_env_file(&update_value.values) {
        return SwarmResponse {
            success: false,
            message: e.to_string(),
            data: None,
        };
    }

    // sync new values into the process environment
    for (key, value) in &update_value.values {
        std::env::set_var(key, value);
    }

    // 2. Stack mutations (brief write lock + save)
    let node_names = config::stack_write(proj, |stack| {
        if update_value.id.as_deref() == Some("boltwall") {
            update_boltwall_env(stack, &mut update_value.values);
        }
        if let Some(host) = update_value.values.get("HOST") {
            for node in stack.nodes.iter_mut() {
                if let Node::Internal(img) = node {
                    img.set_host(host);
                }
            }
        }
        stack.nodes.iter().filter_map(|n| match n {
            Node::Internal(_) => Some(n.name()),
            _ => Some(n.name()),
        }).collect::<Vec<String>>()
    }).await;

    // 3. Docker stop/remove (no lock held)
    let mut error_messages: Vec<String> = Vec::new();
    for name in &node_names {
        match stop_and_remove(docker, &domain(name)).await {
            Ok(_) => log::info!("{} stopped and removed", name),
            Err(e) => {
                error_messages.push(format!("could not stop {}: {}", name, e));
            }
        }
    }

    if !error_messages.is_empty() {
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

fn update_boltwall_env(stack: &mut Stack, env_values: &mut HashMap<String, String>) {
    let mut to_be_updated_env: Vec<KeyToBeUpdated> = Vec::new();
    let nodes: Vec<Node> = stack
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

    stack.nodes = nodes;

    for env in to_be_updated_env {
        if let Some(value) = env_values.get(&env.old_key) {
            env_values.insert(env.new_key, value.clone());
            env_values.remove(&env.old_key);
        }
    }
}

pub async fn handle_assign_reserved_swarm_to_active(
    proj: &str,
    docker: &Docker,
    new_details: &AssignSwarmNewDetails,
    user_id: Option<u32>,
) -> SwarmResponse {
    let mut error_messages: Vec<String> = Vec::new();

    // Step 1: Change password if provided (uses stack_write internally)
    if new_details.new_password.is_some() && new_details.old_password.is_some() {
        let change_password_response = change_swarm_user_password_by_user_admin(
            proj,
            user_id,
            ChangeUserPasswordBySuperAdminInfo {
                new_password: new_details.new_password.clone().unwrap(),
                current_password: new_details.old_password.clone().unwrap(),
                username: "admin".to_string(),
            },
        )
        .await;
        log::info!("Change password response: {:?}", change_password_response);
        if !change_password_response.success {
            error_messages.push(change_password_response.message);
        }
    }

    // Step 2: Persist env vars to .env file and process env (no container restart)
    let mut envs = new_details.env.clone().unwrap_or_default();
    envs.insert("SWARM_ASSIGNED".to_string(), "true".to_string());

    // Stack mutation for boltwall env (brief write lock + save)
    config::stack_write(proj, |stack| {
        update_boltwall_env(stack, &mut envs);
    }).await;

    if let Err(e) = update_or_write_to_env_file(&envs) {
        error_messages.push(e.to_string());
    }
    for (key, value) in &envs {
        std::env::set_var(key, value);
    }

    // Step 3: Set OWNER_PUBKEY on running Boltwall via live API (no lock, HTTP call)
    if let Some(pubkey) = envs.get("OWNER_PUBKEY") {
        let bw = config::stack_read(|s| find_boltwall(&s.nodes)).await;
        match bw {
            Some(bw) => match add_admin_pubkey(&bw, pubkey, "admin").await {
                Ok(_) => log::info!("OWNER_PUBKEY set on boltwall via API"),
                Err(e) => error_messages.push(format!("add_admin_pubkey failed: {}", e)),
            },
            None => error_messages.push("boltwall not found in stack".to_string()),
        }
    }

    // Step 4: If HOST changed, update stack, restart containers
    if let Some(host) = envs.get("HOST") {
        let host = host.clone();
        let shared = envs.get("NAV_BOLTWALL_SHARED_HOST").cloned();
        // Stack mutation (brief write lock + save)
        let node_names = config::stack_write(proj, |stack| {
            stack.host = Some(host.clone());
            if let Some(shared) = shared {
                stack.custom_2b_domain = Some(shared);
            }
            for node in stack.nodes.iter_mut() {
                if let Node::Internal(img) = node {
                    img.set_host(&host);
                }
            }
            stack.nodes.iter().filter_map(|n| match n {
                Node::Internal(_) => Some(n.name()),
                _ => None,
            }).collect::<Vec<String>>()
        }).await;

        // Docker restarts (no lock held)
        for name in &node_names {
            match restart_node_container_global(docker, name, proj).await {
                Ok(_) => log::info!("{} restarted", name),
                Err(e) => {
                    error_messages.push(format!("could not restart {}: {}", name, e));
                }
            }
        }
        match update_swarm().await {
            Ok(_) => log::info!("swarm restart triggered for HOST change"),
            Err(e) => error_messages.push(format!("update_swarm failed: {}", e)),
        }
    }

    if !error_messages.is_empty() {
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

fn find_boltwall(nodes: &[Node]) -> Option<BoltwallImage> {
    nodes.iter().find_map(|n| match n {
        Node::Internal(Image::BoltWall(bw)) => Some(bw.clone()),
        _ => None,
    })
}
