use std::collections::HashMap;

use crate::app_login::sign_up_admin_pubkey;
use crate::auth;
use crate::builder;
use crate::cmd::*;
use crate::config;
use crate::config::LightningPeer;
use crate::config::Role;
use crate::config::User;
use crate::config::{ClientMap, Node, Stack, CLIENTS, STACK};
use crate::conn::boltwall::{
    get_api_token, get_max_request_size, get_request_per_seconds,
    update_max_request_size_config, update_request_per_seconds_config,
};
use crate::conn::swarm::add_new_lightning_peer;
use crate::conn::swarm::handle_assign_reserved_swarm_to_active;
use crate::conn::swarm::update_lightning_peer;
use crate::conn::swarm::SwarmResponse;
use crate::conn::swarm::{
    change_swarm_user_password_by_user_admin, get_image_tags, update_env_variables,
};
use crate::conn::swarm::{create_bot_invoice, get_bot_balance, get_bot_token, get_neo4j_password};
use crate::dock::*;
use crate::dock::restart_node_container_global;
use crate::images::DockerHubImage;
use crate::images::Image;

use crate::rocket_utils::CmdRequest;
use crate::secrets;
use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use rocket::tokio;
use rocket::tokio::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

fn access(cmd: &Cmd, stack: &Stack, user_id: &Option<u32>) -> bool {
    // login needs no auth
    if let Cmd::Swarm(c) = cmd {
        if let SwarmCmd::Login(_) = c {
            return true;
        }
    }
    // user id required if not SwarmCmd::Login
    if user_id.is_none() {
        return false;
    }
    let user_id = user_id.unwrap();
    let user = stack.users.iter().find(|u| u.id == user_id);
    // user required
    if user.is_none() {
        return false;
    }
    match user.unwrap().role {
        Role::Admin => true,
        Role::SubAdmin => true,
        Role::Super => match cmd {
            Cmd::Swarm(c) => match c {
                SwarmCmd::StartContainer(_) => true,
                SwarmCmd::StopContainer(_) => true,
                SwarmCmd::GetConfig => true,
                SwarmCmd::UpdateSwarm => true,
                SwarmCmd::ListContainers => true,
                SwarmCmd::UpdateNode(_) => true,
                SwarmCmd::RestartContainer(_) => true,
                SwarmCmd::GetAllImageActualVersion => true,
                SwarmCmd::ChangePassword(_) => true,
                SwarmCmd::ChangeUserPasswordBySuperAdmin(_) => true,
                SwarmCmd::GetApiToken => true,
                SwarmCmd::ChangeReservedSwarmToActive(_) => true,
                SwarmCmd::UpdateEvn(_) => true,
                SwarmCmd::UpdateNeo4jConfig(_) => true,
                _ => false,
            },
            _ => false,
        },
    }
}

// tag is the service name
pub async fn handle(
    proj: &str,
    cmd: Cmd,
    tag: &str,
    docker: &Docker,
    user_id: &Option<u32>,
) -> Result<String> {
    // Access check uses a brief read lock
    let allowed = config::stack_read(|s| access(&cmd, s, user_id)).await;
    if !allowed {
        return Err(anyhow!("access denied"));
    }

    let ready = config::stack_read(|s| s.ready).await;
    if !ready && !cmd.can_run_before_ready() {
        return Err(anyhow!("cant run this command yet..."));
    }

    log::info!("=> CMD: {:?}", cmd);

    let ret: Option<String> = match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::GetConfig => {
                let res = config::stack_read(|s| serde_json::to_string(&s.remove_tokens())).await?;
                Some(res)
            }
            SwarmCmd::StartContainer(id) => {
                log::info!("StartContainer -> {}", id);
                let res = start_container(docker, &id).await?;
                let img = config::stack_read(|s| {
                    builder::find_image_by_hostname(&s.nodes, &id)
                }).await?;
                if let Err(e) = img.post_startup(proj, docker).await {
                    log::warn!("{:?}", e);
                }
                let cm = CLIENTS.read().await;
                img.post_client(&cm).await?;
                drop(cm);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::StopContainer(id) => {
                log::info!("StopContainer -> {}", id);
                let res = stop_container(docker, &id).await?;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::RestartContainer(id) => {
                log::info!("RestartContainer -> {}", id);
                restart_node_container_global(docker, &id, proj).await?;
                Some(serde_json::to_string("{}")?)
            }
            SwarmCmd::AddNode(node) => {
                log::info!("AddNode -> {:?}", node);
                // add a node via docker
                None
            }
            SwarmCmd::UpdateNode(un) => {
                log::info!("UpdateNode -> {}", un.id);
                config::stack_write(proj, |s| {
                    for node in s.nodes.iter_mut() {
                        if node.name() == un.id {
                            let _ = node.set_version(&un.version);
                        }
                    }
                }).await;
                // update_node_from_state handles Docker work + client reconnect via globals
                builder::update_node_from_state(proj, docker, &un.id).await?;
                Some(serde_json::to_string("{}")?)
            }
            SwarmCmd::GetContainerLogs(container_name) => {
                let logs = container_logs(docker, &container_name).await;
                Some(serde_json::to_string(&logs)?)
            }
            SwarmCmd::ListVersions(req) => {
                #[derive(Serialize, Deserialize, Debug, Clone)]
                struct ListVersionsResult {
                    org: String,
                    repo: String,
                    images: String,
                }
                let img = config::stack_read(|s| {
                    s.nodes.iter()
                        .find(|n| n.name() == req.name)
                        .and_then(|n| n.as_internal().ok())
                        .map(|i| i.repo())
                }).await.context(format!("cant find node {}", &req.name))?;
                let url = format!(
                    "https://hub.docker.com/v2/namespaces/{}/repositories/{}/tags?page={}",
                    img.org, img.repo, req.page
                );
                let body = reqwest::get(url).await?.text().await?;
                Some(serde_json::to_string(&ListVersionsResult {
                    org: img.org.clone(),
                    repo: img.repo.clone(),
                    images: body,
                })?)
            }
            SwarmCmd::Login(ld) => {
                // Pattern 7: read user data, drop lock, bcrypt outside
                let user_data = config::stack_read(|s| {
                    s.users.iter().find(|u| u.username == ld.username)
                        .map(|u| (u.id, u.pass_hash.clone()))
                }).await;
                match user_data {
                    Some((uid, hash)) => {
                        if !bcrypt::verify(&ld.password, &hash)? {
                            Some("".to_string())
                        } else {
                            let mut hm = HashMap::new();
                            hm.insert("token", auth::make_jwt(uid)?);
                            Some(serde_json::to_string(&hm)?)
                        }
                    }
                    None => Some("".to_string()),
                }
            }
            SwarmCmd::ChangePassword(cp) => {
                // Pattern 7: read hash, bcrypt outside, write back
                let user_data = config::stack_read(|s| {
                    s.users.iter().find(|u| u.id == cp.user_id)
                        .map(|u| u.pass_hash.clone())
                }).await;
                match user_data {
                    Some(old_hash) => {
                        if bcrypt::verify(&cp.old_pass, &old_hash)? {
                            let new_hash = bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                            config::stack_write(proj, |s| {
                                if let Some(u) = s.users.iter_mut().find(|u| u.id == cp.user_id) {
                                    u.pass_hash = new_hash;
                                }
                            }).await;
                            let mut hm = HashMap::new();
                            hm.insert("success", true);
                            Some(serde_json::to_string(&hm)?)
                        } else {
                            Some("".to_string())
                        }
                    }
                    None => Some("".to_string()),
                }
            }
            SwarmCmd::ChangeAdmin(cp) => {
                let user_data = config::stack_read(|s| {
                    s.users.iter().find(|u| u.id == cp.user_id)
                        .map(|u| u.pass_hash.clone())
                }).await;
                match user_data {
                    Some(old_hash) => {
                        if bcrypt::verify(&cp.old_pass, &old_hash)? {
                            let new_hash = bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                            config::stack_write(proj, |s| {
                                if let Some(u) = s.users.iter_mut().find(|u| u.id == cp.user_id) {
                                    u.pass_hash = new_hash;
                                    u.username = cp.email.clone();
                                }
                            }).await;
                            let mut hm = HashMap::new();
                            hm.insert("success", true);
                            Some(serde_json::to_string(&hm)?)
                        } else {
                            Some("".to_string())
                        }
                    }
                    None => Some("".to_string()),
                }
            }
            SwarmCmd::ListContainers => {
                let containers = list_containers(docker).await?;
                Some(serde_json::to_string(&containers)?)
            }
            SwarmCmd::GetStatistics(container_name) => {
                let docker = dockr();
                println!("Calling GetStatistics with {:?}", &container_name);
                let containers = get_container_statistics(&docker, container_name).await?;
                println!("GetStatistics Called");
                Some(serde_json::to_string(&containers)?)
            }
            SwarmCmd::AddBoltwallAdminPubkey(admin) => {
                log::info!("AddBoltwallAdminPubkey ->pubkey {}, name {:?}", admin.pubkey, admin.name);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let name = admin.name.unwrap_or("".to_string());
                let response = crate::conn::boltwall::add_admin_pubkey(&boltwall, &admin.pubkey, &name).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::GetBoltwallSuperAdmin => {
                log::info!("GetBoltwallSuperAdmin");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::get_super_admin(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::AddBoltwallUser(user) => {
                log::info!("AddBoltwallUser -> pubkey {}-> role {} -> name {:?} ", user.pubkey, user.role, user.name);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let name = user.name.unwrap_or("".to_string());
                // HTTP call with no lock held
                let (response_text, status) = crate::conn::boltwall::add_user_http(&boltwall, &user.pubkey, user.role, &name).await?;
                if status == 200 || status == 201 {
                    // Brief stack write to sync user
                    let changed = config::stack_write(proj, |s| {
                        crate::conn::boltwall::add_user_to_stack(user.role, user.pubkey.clone(), name.clone(), s)
                    }).await;
                    let _ = changed; // save already happened in stack_write
                }
                Some(serde_json::to_string(&response_text)?)
            }
            SwarmCmd::ListAdmins => {
                log::info!("ListAdmins ==> ");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::list_admins(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::DeleteSubAdmin(apk) => {
                log::info!("DeleteSubAdmin -> {}", apk);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                // HTTP call with no lock held
                let (response_text, status) = crate::conn::boltwall::delete_sub_admin_http(&boltwall, &apk).await?;
                if status == 200 {
                    let changed = config::stack_write(proj, |s| {
                        crate::conn::boltwall::delete_sub_admin_from_stack(apk.clone(), s)
                    }).await;
                    let _ = changed;
                }
                Some(serde_json::to_string(&response_text)?)
            }
            SwarmCmd::ListPaidEndpoint => {
                log::info!("ListPaidEndpoint ===> ");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::list_paid_endpoint(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateSwarm => {
                log::info!("Updating Swarm ===>");
                let response = crate::conn::swarm::update_swarm().await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdatePaidEndpoint(details) => {
                log::info!("UpdatePaidEndpoint -> Status:{} ID:{}", details.status, details.id);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::update_paid_endpoint(&boltwall, details.id, details.status).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateEndpointPrice(details) => {
                log::info!("UpdateEndpointPrice -> ID:{} Price:{}", details.id, details.price);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::update_endpoint_price(&boltwall, details.id, details.price).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateBoltwallAccessibility(is_public) => {
                log::info!("UpdateBoltwallAccessibility -> Status:{} ", is_public);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::update_boltwall_accessibility(&boltwall, is_public).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::GetBoltwallAccessibility => {
                log::info!("Get Boltwall Accessibility ===>");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::get_boltwall_accessibility(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateAdminPubkey(details) => {
                let res = config::stack_write(proj, |s| {
                    match s.users.iter().position(|u| u.id == details.user_id) {
                        Some(ui) => {
                            s.users[ui].pubkey = Some(details.pubkey.to_string());
                            let mut hm = HashMap::new();
                            hm.insert("success", true);
                            serde_json::to_string(&hm).unwrap_or_default()
                        }
                        None => "invalid user".to_string(),
                    }
                }).await;
                Some(res)
            }
            SwarmCmd::GetFeatureFlags => {
                log::info!("Get Boltwall Feature Flags ===>");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::get_feature_flags(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::GetSecondBrainAboutDetails => {
                log::info!("Get Second Brain About Details ===>");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::get_second_brain_about_details(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateSecondBrainAbout(about) => {
                log::info!("Update Second Brain Title: {:?}", about.title);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::update_second_brain_about(&boltwall, about).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateFeatureFlags(body) => {
                log::info!("Update Feature Flags ===> {:?}", body);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = crate::conn::boltwall::update_feature_flags(&boltwall, body).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::SignUpAdminPubkey(body) => {
                log::info!("Signup Admin Pubkey ===> {:?}", body);
                let response = sign_up_admin_pubkey(proj, body).await?;
                return Ok(serde_json::to_string(&response)?);
            }
            SwarmCmd::GetImageDigest(image_name) => {
                let digest = get_image_digest(&image_name).await?;
                return Ok(serde_json::to_string(&digest)?);
            }
            SwarmCmd::GetDockerImageTags(image_details) => {
                log::info!("Get Docker Image Tags ===> {:?}", image_details);
                let tags = get_image_tags(image_details).await?;
                return Ok(serde_json::to_string(&tags)?);
            }
            SwarmCmd::GetAllImageActualVersion => {
                log::info!("Get all Image actual version");
                let nodes = config::stack_read(|s| s.nodes.clone()).await;
                let image_versions = get_image_actual_version(&nodes).await?;
                return Ok(serde_json::to_string(&image_versions)?);
            }
            SwarmCmd::UpdateUser(body) => {
                log::info!("Update users details ===> {:?}", body);
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                // HTTP call with no lock held
                let (response_text, status) = crate::conn::boltwall::update_user_http(&boltwall, &body.pubkey, &body.name, body.id, body.role).await?;
                if status == 200 {
                    config::stack_write(proj, |s| {
                        crate::conn::boltwall::add_user_to_stack(body.role, body.pubkey.clone(), body.name.clone(), s);
                    }).await;
                }
                Some(serde_json::to_string(&response_text)?)
            }
            SwarmCmd::GetApiToken => {
                log::info!("Get API TOKEN");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let response = get_api_token(&boltwall).await?;
                return Ok(serde_json::to_string(&response)?);
            }
            SwarmCmd::SetGlobalMemLimit(gbm) => {
                config::stack_write(proj, |s| { s.global_mem_limit = Some(gbm); }).await;
                Some(crate::config::set_global_mem_limit(gbm)?)
            }
            SwarmCmd::GetSignedInUserDetails => {
                log::info!("Get Signed In Users details");
                if user_id.is_none() {
                    return Ok("invalid user".to_string());
                }
                let uid = user_id.unwrap();
                let res = config::stack_read(|s| {
                    match s.users.iter().find(|user| user.id == uid) {
                        Some(user) => {
                            let modified_user = User {
                                pass_hash: "".to_string(),
                                username: user.username.clone(),
                                id: user.id,
                                pubkey: user.pubkey.clone(),
                                role: user.role.clone(),
                            };
                            serde_json::to_string(&modified_user).ok()
                        }
                        None => Some("invalid user".to_string()),
                    }
                }).await;
                Some(res.unwrap_or("invalid user".to_string()))
            }
            SwarmCmd::ChangeUserPasswordBySuperAdmin(info) => {
                log::info!("Change user password from superadmin");
                let res = change_swarm_user_password_by_user_admin(proj, user_id.clone(), info).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetLightningPeers => {
                log::info!("Get all lightning peers");
                let res = config::stack_read(|s| serde_json::to_string(&s.lightning_peers)).await?;
                Some(res)
            }
            SwarmCmd::AddLightningPeer(info) => {
                log::info!("Add new lightning peer");
                let res = config::stack_write(proj, |s| {
                    add_new_lightning_peer(s, info)
                }).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateLightningPeer(info) => {
                log::info!("Update Lightning peer");
                let res = config::stack_write(proj, |s| {
                    update_lightning_peer(s, info)
                }).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetNeo4jPassword => {
                log::info!("Get Neo4j Password");
                let res = config::stack_read(|s| get_neo4j_password(&s.nodes)).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetBotToken => {
                log::info!("Get Bot Token");
                let res = config::stack_read(|s| get_bot_token(&s.nodes)).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetBotBalance => {
                log::info!("Get Bot Balance");
                let nodes = config::stack_read(|s| s.nodes.clone()).await;
                let res = get_bot_balance(&nodes).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::CreateBotInvoice(body) => {
                log::info!("Create Bot Invoice");
                let nodes = config::stack_read(|s| s.nodes.clone()).await;
                let res = create_bot_invoice(&nodes, body.amt_msat).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetL402Stats => {
                log::info!("Get L402 Stats");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let res = crate::conn::boltwall::get_l402_stats(&boltwall).await?;
                Some(res)
            }
            SwarmCmd::GetAdminTransactions(params) => {
                log::info!("Get Admin Transactions");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let res = crate::conn::boltwall::get_admin_transactions(&boltwall, &params).await?;
                Some(res)
            }
            SwarmCmd::UpdateBoltwallRequestPerSeconds(info) => {
                log::info!("Update Boltwall Request per seconds to: {}", &info.request_per_seconds);
                let res = config::stack_write(proj, |s| {
                    update_request_per_seconds_config(info.request_per_seconds, s)
                }).await;
                if res.success {
                    let boltwall_name = config::stack_read(|s| {
                        find_boltwall(&s.nodes).ok().map(|b| b.name)
                    }).await;
                    if let Some(name) = boltwall_name {
                        if let Err(e) = restart_node_container_global(docker, &name, proj).await {
                            log::error!("Error restarting boltwall: {}", e);
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetBoltwallRequestPerSeconds => {
                log::info!("Get Boltwall Request Per Seconds");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let res = get_request_per_seconds(&boltwall);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetEnv(id) => {
                log::info!("Get {} env variables", id);
                let res = get_env_variables_by_container_name(&docker, &id).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetBoltwallMaxRequestLimit => {
                log::info!("Get Boltwall Max Request Limit");
                let boltwall = config::stack_read(|s| find_boltwall(&s.nodes)).await?;
                let res = get_max_request_size(&boltwall);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateBoltwallMaxRequestLimit(info) => {
                log::info!("Update Boltwall Max Request Limit: {}", &info.max_request_limit);
                let res = config::stack_write(proj, |s| {
                    update_max_request_size_config(&info.max_request_limit, s)
                }).await;
                if res.success {
                    let boltwall_name = config::stack_read(|s| {
                        find_boltwall(&s.nodes).ok().map(|b| b.name)
                    }).await;
                    if let Some(name) = boltwall_name {
                        if let Err(e) = restart_node_container_global(docker, &name, proj).await {
                            log::error!("Error restarting boltwall: {}", e);
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateEvn(update_env) => {
                log::info!("Update env variables for {:#?}", update_env.id);
                let res = update_env_variables(proj, &docker, &mut update_env.clone()).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::ChangeReservedSwarmToActive(details) => {
                log::info!("About to update reserved swarm to active");
                let res = handle_assign_reserved_swarm_to_active(
                    proj, &docker, &details, user_id.clone()
                ).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateSslCert => {
                let res = match crate::renew_ssl_cert::handle_update_ssl_cert(proj).await {
                    Ok(_) => SwarmResponse { success: true, message: "ssl certificate updated successfully".to_string(), data: None },
                    Err(err) => SwarmResponse { success: false, message: err.to_string(), data: None },
                };
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateNeo4jConfig(body) => {
                log::info!("Update Neo4j config ===> {:?}", body);
                let res = config::stack_write(proj, |s| {
                    let mut updated = false;
                    for node in s.nodes.iter_mut() {
                        if let Node::Internal(Image::Neo4j(ref mut neo)) = node {
                            if let Some(v) = body.heap_initial_gb { neo.heap_initial_gb = Some(v); }
                            if let Some(v) = body.heap_max_gb { neo.heap_max_gb = Some(v); }
                            if let Some(v) = body.pagecache_gb { neo.pagecache_gb = Some(v); }
                            if let Some(v) = body.tx_total_gb { neo.tx_total_gb = Some(v); }
                            if let Some(v) = body.tx_max_gb { neo.tx_max_gb = Some(v); }
                            if let Some(v) = body.checkpoint_iops { neo.checkpoint_iops = Some(v); }
                            updated = true;
                        }
                    }
                    if updated {
                        SwarmResponse { success: true, message: "neo4j config updated; restart neo4j container to apply changes".to_string(), data: None }
                    } else {
                        SwarmResponse { success: false, message: "neo4j image not found in stack".to_string(), data: None }
                    }
                }).await;
                Some(serde_json::to_string(&res)?)
            }
        },
        // Pattern 5: Client only — clone out and call
        Cmd::Relay(c) => {
            let client = config::clients_read(|c| c.relay.get(tag).cloned()).await.context("no relay client")?;
            match c {
                RelayCmd::AddUser(u) => Some(client.add_user(u.initial_sats).await?.to_string()?),
                RelayCmd::ListUsers => Some(client.list_users().await?.to_string()?),
                RelayCmd::GetChats => Some(client.get_chats().await?.to_string()?),
                RelayCmd::AddDefaultTribe(t) => Some(client.add_default_tribe(t.id).await?.to_string()?),
                RelayCmd::RemoveDefaultTribe(t) => Some(client.remove_default_tribe(t.id).await?.to_string()?),
                RelayCmd::GetToken => {
                    let secs = secrets::load_secrets(proj).await;
                    let token = secs.get(tag).context("no relay token")?;
                    let mut hm = HashMap::new();
                    hm.insert("token", base64::encode(token));
                    Some(serde_json::to_string(&hm)?)
                }
                RelayCmd::GetBalance => Some(client.get_balance().await?.to_string()?),
            }
        }
        Cmd::Bitcoind(c) => {
            let client = config::clients_read(|c| c.bitcoind.get(tag).cloned()).await.context("no bitcoind client")?;
            match c {
                BitcoindCmd::GetInfo => {
                    let info = client.get_info()?;
                    Some(serde_json::to_string(&info)?)
                }
                BitcoindCmd::TestMine(tm) => {
                    let res = client.test_mine(tm.blocks, tm.address)?;
                    Some(serde_json::to_string(&res)?)
                }
                BitcoindCmd::GetBalance => {
                    let res = client.get_wallet_balance()?;
                    Some(serde_json::to_string(&res)?)
                }
            }
        }
        Cmd::Lnd(c) => {
            let client = config::clients_read(|c| c.lnd.get(tag).cloned()).await.context("no lnd client")?;
            match c {
                LndCmd::GetInfo => {
                    let info = client.lock().await.get_info().await?;
                    Some(serde_json::to_string(&info)?)
                }
                LndCmd::ListChannels => {
                    let channel_list = client.lock().await.list_channels().await?;
                    Some(serde_json::to_string(&channel_list.channels)?)
                }
                LndCmd::AddPeer(peer) => {
                    if let Some(alias) = peer.alias.clone() {
                        config::stack_write(proj, |s| {
                            add_new_lightning_peer(s, LightningPeer { pubkey: peer.pubkey.clone(), alias });
                        }).await;
                    }
                    let result = client.lock().await.add_peer(peer).await?;
                    Some(serde_json::to_string(&result)?)
                }
                LndCmd::ListPeers => {
                    let result = client.lock().await.list_peers().await?;
                    Some(serde_json::to_string(&result)?)
                }
                LndCmd::AddChannel(channel) => {
                    let channel = client.lock().await.create_channel(channel).await?;
                    Some(serde_json::to_string(&channel)?)
                }
                LndCmd::NewAddress => {
                    let address = client.lock().await.new_address().await?;
                    Some(serde_json::to_string(&address.address)?)
                }
                LndCmd::GetBalance => {
                    let bal = client.lock().await.get_balance().await?;
                    Some(serde_json::to_string(&bal)?)
                }
                LndCmd::AddInvoice(invoice) => {
                    let invoice = client.lock().await.add_invoice(invoice).await?;
                    Some(serde_json::to_string(&invoice)?)
                }
                LndCmd::PayInvoice(invoice) => {
                    let invoice = client.lock().await.pay_invoice(invoice).await?;
                    Some(serde_json::to_string(&invoice)?)
                }
                LndCmd::PayKeysend(keysend) => {
                    let invoice = client.lock().await.pay_keysend(keysend).await?;
                    Some(serde_json::to_string(&invoice)?)
                }
                LndCmd::ListPayments => {
                    let payments = client.lock().await.list_payments().await?;
                    Some(serde_json::to_string(&payments)?)
                }
                LndCmd::ListInvoices => {
                    let invoices = client.lock().await.list_invoices().await?;
                    Some(serde_json::to_string(&invoices)?)
                }
                LndCmd::ListPendingChannels => {
                    let pending_channel_list = client.lock().await.list_pending_channels().await?;
                    Some(serde_json::to_string(&pending_channel_list.pending_open_channels)?)
                }
            }
        }
        Cmd::Cln(c) => {
            let client = config::clients_read(|c| c.cln.get(tag).cloned()).await.context("no cln client")?;
            match c {
                ClnCmd::GetInfo => {
                    let info = client.get_info().await?;
                    Some(serde_json::to_string(&info)?)
                }
                ClnCmd::ListPeers => {
                    let info = client.list_peers().await?;
                    Some(serde_json::to_string(&info)?)
                }
                ClnCmd::ListPeerChannels => {
                    let info = client.list_peer_channels(None).await?;
                    Some(serde_json::to_string(&info)?)
                }
                ClnCmd::ListFunds => {
                    let funds = client.list_funds().await?;
                    Some(serde_json::to_string(&funds)?)
                }
                ClnCmd::NewAddress => {
                    let address = client.new_addr().await?;
                    Some(serde_json::to_string(&address.bech32.unwrap_or("".to_string()))?)
                }
                ClnCmd::AddPeer(peer) => {
                    let mut port = "9735";
                    let hsplit = peer.host.clone();
                    let host = if let Some((addr, p)) = hsplit.split_once(":") {
                        port = p;
                        addr.to_string()
                    } else {
                        peer.host
                    };
                    if let Some(alias) = peer.alias.clone() {
                        config::stack_write(proj, |s| {
                            add_new_lightning_peer(s, LightningPeer { alias, pubkey: peer.pubkey.clone() });
                        }).await;
                    }
                    let result = client.connect_peer(&peer.pubkey, &host, port).await?;
                    Some(serde_json::to_string(&result)?)
                }
                ClnCmd::AddChannel(channel) => {
                    let channel = client.fund_channel(&channel.pubkey, channel.amount.try_into()?, Some(channel.satsperbyte.try_into()?)).await?;
                    Some(serde_json::to_string(&channel)?)
                }
                ClnCmd::AddInvoice(i) => {
                    let inv = client.create_invoice(i.amt_paid_sat as u64).await?;
                    Some(serde_json::to_string(&inv)?)
                }
                ClnCmd::PayInvoice(i) => {
                    let paid = client.pay(&i.payment_request).await?;
                    Some(serde_json::to_string(&paid)?)
                }
                ClnCmd::PayKeysend(i) => {
                    let paid = client.keysend(&i.dest, i.amt as u64, i.route_hint, i.maxfeepercent, i.exemptfee, None).await?;
                    Some(serde_json::to_string(&paid)?)
                }
                ClnCmd::CloseChannel(i) => {
                    let closed = client.close(&i.id, &i.destination).await?;
                    let mut hm = HashMap::new();
                    hm.insert("type", closed.item_type.to_string());
                    hm.insert("txid", hex::encode(closed.txid()));
                    hm.insert("tx", hex::encode(closed.tx()));
                    Some(serde_json::to_string(&hm)?)
                }
                ClnCmd::ListInvoices(i) => match i {
                    Some(hash) => {
                        let invoices = client.list_invoices(hash.payment_hash).await?;
                        Some(serde_json::to_string(&invoices)?)
                    }
                    None => {
                        let invoices = client.list_invoices(None).await?;
                        Some(serde_json::to_string(&invoices)?)
                    }
                },
                ClnCmd::ListPays(i) => match i {
                    Some(hash) => {
                        let pays = client.list_pays(hash.payment_hash).await?;
                        Some(serde_json::to_string(&pays)?)
                    }
                    None => {
                        let pays = client.list_pays(None).await?;
                        Some(serde_json::to_string(&pays)?)
                    }
                },
            }
        }
        Cmd::Proxy(c) => {
            let client = config::clients_read(|c| c.proxy.get(tag).cloned()).await.context("no proxy client")?;
            match c {
                ProxyCmd::GetBalance => {
                    let balance = client.get_balance().await?;
                    Some(serde_json::to_string(&balance)?)
                }
            }
        }
        Cmd::Hsmd(c) => {
            let client = config::clients_read(|c| c.hsmd.get(tag).cloned()).await.context("no cln for hsmd client")?;
            match c {
                HsmdCmd::GetClients => {
                    let clients = client.get_clients().await?;
                    Some(serde_json::to_string(&clients)?)
                }
            }
        }
    };

    Ok(ret.context("internal error")?)
}

use crate::images::boltwall::BoltwallImage;
pub fn find_boltwall(nodes: &Vec<Node>) -> Result<BoltwallImage> {
    let mut boltwall_opt = None;
    for img in nodes.iter() {
        if let Ok(ii) = img.as_internal() {
            if let Ok(boltwall) = ii.as_boltwall() {
                boltwall_opt = Some(boltwall);
            }
        }
    }
    Ok(boltwall_opt.context(anyhow!("no boltwall image"))?)
}

pub async fn hydrate(mut stack: Stack, clients: ClientMap) {
    // set into the globals
    stack.ready = true;
    let mut sg = STACK.write().await;
    *sg = stack;
    drop(sg);
    let mut cg = CLIENTS.write().await;
    *cg = clients;
}

pub async fn hydrate_stack(stack: Stack) {
    let mut sg = STACK.write().await;
    *sg = stack;
}

pub async fn hydrate_clients(clients: ClientMap) {
    let mut cg = CLIENTS.write().await;
    *cg = clients;
    drop(cg);
    let mut sg = STACK.write().await;
    sg.ready = true;
}

pub fn spawn_handler(proj: &str, mut rx: mpsc::Receiver<CmdRequest>, docker: Docker) {
    let project = proj.to_string();
    let timeout_duration =
        std::env::var("REQUEST_TIMEOUT_DURATION_IN_SEC").unwrap_or_else(|_| "60".to_string());
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let response = if let Ok(cmd) = serde_json::from_str::<Cmd>(&msg.message) {
                match tokio::time::timeout(
                    Duration::from_secs(timeout_duration.parse().unwrap_or(60)),
                    handle(&project, cmd, &msg.tag, &docker, &msg.user_id),
                )
                .await
                {
                    Ok(Ok(res)) => res,
                    Ok(Err(err)) => {
                        log::warn!("handle ERR {:?}", err);
                        fmt_err(&err.to_string())
                    }
                    Err(_error) => fmt_err("Handle operation timed out"),
                }
            } else {
                fmt_err("Invalid Command")
            };
            let _ = msg.reply_tx.send(response);
        }
    });
}

fn fmt_err(err: &str) -> String {
    format!("{{\"stack_error\":\"{}\"}}", err.to_string())
}
