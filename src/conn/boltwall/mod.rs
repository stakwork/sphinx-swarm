use crate::cmd::FeatureFlagUserRoles;
use crate::config::{Node, Role, State, User};
use crate::dock::restart_node_container;
use crate::images::Image;
use crate::utils::docker_domain;
use crate::{cmd::UpdateSecondBrainAboutRequest, images::boltwall::BoltwallImage};
use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use super::swarm::SwarmResponse;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetAdminPubkeyBody {
    pub pubkey: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddUserBody {
    pubkey: String,
    role: u32,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatePaidEndpointBody {
    id: u64,
    status: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiToken {
    pub x_api_token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoltwallAccessibility {
    is_public: bool,
}

fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .danger_accept_invalid_certs(true)
        .build()
        .expect("couldnt build boltwall reqwest client")
}

pub async fn add_admin_pubkey(img: &BoltwallImage, pubkey: &str, name: &str) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/set_admin_pubkey", host, img.port);

    let body = SetAdminPubkeyBody {
        pubkey: pubkey.to_string(),
        name: name.to_string(),
    };
    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_super_admin(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);
    let route = format!("http://{}:{}/super_admin", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn add_user(
    img: &BoltwallImage,
    pubkey: &str,
    role: u32,
    name: String,
    state: &mut State,
    must_save_stack: &mut bool,
) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/set_user_role", host, img.port);

    let body = AddUserBody {
        pubkey: pubkey.to_string(),
        role: role,
        name: name.to_string(),
    };
    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    if response.status() == 200 || response.status() == 201 {
        // handle add user to swarm user
        let did_update_user = add_or_edit_user(body.role, body.pubkey, body.name, state);
        if did_update_user == true {
            *must_save_stack = true
        }
    }

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn list_admins(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/admins", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn delete_sub_admin(
    img: &BoltwallImage,
    pubkey: &str,
    state: &mut State,
    must_save_stack: &mut bool,
) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/user/{}", host, img.port, pubkey);

    let response = client
        .delete(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    if response.status() == 200 {
        let update_state = delete_swarm_user(state, pubkey.to_string());
        if update_state == true {
            *must_save_stack = true
        }
    }

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn list_paid_endpoint(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/endpointsList", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn update_paid_endpoint(img: &BoltwallImage, id: u64, status: bool) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/updateEndpointStatus", host, img.port);

    let body = UpdatePaidEndpointBody {
        id: id,
        status: status,
    };
    let response = client
        .put(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn update_boltwall_accessibility(img: &BoltwallImage, is_public: bool) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/setPublicPrivate", host, img.port);

    let body = UpdateBoltwallAccessibility { is_public };
    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_boltwall_accessibility(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/getPublicPrivate", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_feature_flags(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/featureFlags", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_second_brain_about_details(img: &BoltwallImage) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/about", host, img.port);

    let response = client
        .get(route.as_str())
        .header("x-admin-token", admin_token)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}
pub async fn update_feature_flags(
    img: &BoltwallImage,
    body: HashMap<String, FeatureFlagUserRoles>,
) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/featureFlags", host, img.port);

    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn update_second_brain_about(
    img: &BoltwallImage,
    body: UpdateSecondBrainAboutRequest,
) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/about", host, img.port);

    let response = client
        .post(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn update_user(
    img: &BoltwallImage,
    pubkey: String,
    name: String,
    id: u32,
    role: u32,
    state: &mut State,
    must_save_stack: &mut bool,
) -> Result<String> {
    let admin_token = img.admin_token.clone().context(anyhow!("No admin token"))?;

    let client = make_client();
    let host = docker_domain(&img.name);

    let route = format!("http://{}:{}/user/{}", host, img.port, id);

    let body = AddUserBody {
        pubkey: pubkey.to_string(),
        name: name.to_string(),
        role: role,
    };
    let response = client
        .put(route.as_str())
        .header("x-admin-token", admin_token)
        .json(&body)
        .send()
        .await?;

    if response.status() == 200 {
        let must_update_state = add_or_edit_user(role, pubkey, name, state);
        if must_update_state == true {
            *must_save_stack = true
        }
    }

    let response_text = response.text().await?;

    Ok(response_text)
}

pub async fn get_api_token(boltwall: &BoltwallImage) -> Result<ApiToken> {
    let api_token = boltwall
        .stakwork_secret
        .clone()
        .context(anyhow!("No admin token"))?;

    let response = ApiToken {
        x_api_token: api_token,
    };

    Ok(response)
}

fn add_or_edit_user(role: u32, pubkey: String, name: String, state: &mut State) -> bool {
    return match state
        .stack
        .users
        .iter()
        .position(|u| u.pubkey == Some(pubkey.clone()))
    {
        Some(user_pos) => {
            let cloned_user = state.stack.users.clone();
            // check if role is boltwall member
            if role == 3
                && cloned_user[user_pos].role != Role::Admin
                && cloned_user[user_pos].role != Role::Super
            {
                state.stack.users.remove(user_pos);
                return true;
            }
            false
        }
        None => {
            // check if role is boltwall subadmin
            if role == 2 {
                let new_id = match state.stack.users.last() {
                    Some(last_user) => last_user.id + 1,
                    None => 1, // This block of code should never execute because one user must exist before you can add a user
                };
                state.stack.users.push(User {
                    username: name.to_lowercase(),
                    id: new_id,
                    pubkey: Some(pubkey.clone()),
                    role: Role::SubAdmin,
                    pass_hash: bcrypt::hash(crate::secrets::hex_secret_32(), bcrypt::DEFAULT_COST)
                        .expect("failed to bcrypt"),
                });
                return true;
            }
            false
        }
    };
}

fn delete_swarm_user(state: &mut State, pubkey: String) -> bool {
    match state
        .stack
        .users
        .iter()
        .position(|user| user.pubkey == Some(pubkey.clone()))
    {
        Some(user_pos) => {
            let cloned_user = state.stack.users[user_pos].clone();
            if cloned_user.role != Role::Admin && cloned_user.role != Role::Super {
                state.stack.users.remove(user_pos);
                return true;
            }
            false
        }
        None => false,
    }
}

pub async fn update_request_per_seconds(
    rps: i64,
    state: &mut State,
    must_save_stack: &mut bool,
    docker: &Docker,
    proj: &str,
) -> SwarmResponse {
    if rps < 1 {
        return SwarmResponse {
            success: false,
            message: "request per seconds cannot be less than 1".to_string(),
            data: None,
        };
    }

    let mut node_name = "".to_string();

    let nodes: Vec<Node> = state
        .stack
        .nodes
        .iter()
        .map(|n| match n {
            Node::External(e) => Node::External(e.clone()),
            Node::Internal(i) => match i.clone() {
                Image::BoltWall(mut b) => {
                    b.set_request_per_seconds(rps);
                    node_name = b.name.clone();
                    Node::Internal(Image::BoltWall(b))
                }
                _ => Node::Internal(i.clone()),
            },
        })
        .collect();

    state.stack.nodes = nodes;

    // restart node
    match restart_node_container(docker, node_name.as_str(), state, proj).await {
        Ok(_) => {
            *must_save_stack = true;
            SwarmResponse {
                success: true,
                message: "Request per Seconds updated successfully".to_string(),
                data: None,
            }
        }
        Err(err) => {
            log::error!(
                "Error updating boltwall request per seconds: {}",
                err.to_string()
            );
            SwarmResponse {
                success: false,
                message: format!(
                    "Error updating boltwall request per seconds: {}",
                    err.to_string()
                ),
                data: None,
            }
        }
    }
}

pub fn get_request_per_seconds(boltwall: &BoltwallImage) -> SwarmResponse {
    let mut settled_rps: i64 = 50;

    if let Some(rps) = boltwall.request_per_seconds {
        settled_rps = rps
    }

    SwarmResponse {
        success: true,
        message: "boltwall request per seconds".to_string(),
        data: Some(serde_json::Value::Number(serde_json::Number::from(
            settled_rps,
        ))),
    }
}

pub fn get_max_request_size(boltwall: &BoltwallImage) -> SwarmResponse {
    let mut settled_mrl: String = "1mb".to_string();

    if let Some(mrl) = &boltwall.max_request_limit {
        settled_mrl = mrl.to_string();
    }

    SwarmResponse {
        success: true,
        message: "boltwall max request limit".to_string(),
        data: Some(serde_json::Value::String(settled_mrl)),
    }
}

pub async fn update_max_request_size(
    mrl: &str,
    state: &mut State,
    must_save_stack: &mut bool,
    docker: &Docker,
    proj: &str,
) -> SwarmResponse {
    if mrl.is_empty() {
        return SwarmResponse {
            success: false,
            message: "max request limit cannot be empty".to_string(),
            data: None,
        };
    }

    let mut node_name = "".to_string();

    let nodes: Vec<Node> = state
        .stack
        .nodes
        .iter()
        .map(|n| match n {
            Node::External(e) => Node::External(e.clone()),
            Node::Internal(i) => match i.clone() {
                Image::BoltWall(mut b) => {
                    b.set_max_request_limit(mrl);
                    node_name = b.name.clone();
                    Node::Internal(Image::BoltWall(b))
                }
                _ => Node::Internal(i.clone()),
            },
        })
        .collect();

    state.stack.nodes = nodes;

    // restart node
    match restart_node_container(docker, node_name.as_str(), state, proj).await {
        Ok(_) => {
            *must_save_stack = true;
            SwarmResponse {
                success: true,
                message: "Max request limit updated successfully".to_string(),
                data: None,
            }
        }
        Err(err) => {
            log::error!(
                "Error updating boltwall max request limit: {}",
                err.to_string()
            );
            SwarmResponse {
                success: false,
                message: format!(
                    "Error updating boltwall max request limit: {}",
                    err.to_string()
                ),
                data: None,
            }
        }
    }
}
