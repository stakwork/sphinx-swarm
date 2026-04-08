use std::collections::HashMap;

use crate::app_login::sign_up_admin_pubkey;
use crate::auth;
use crate::builder;
use crate::cmd::*;
use crate::config::{self, Clients, Node, Stack, CLIENTS, STACK};
use crate::config::LightningPeer;
use crate::config::Role;
use crate::config::User;
use crate::conn::boltwall::{
    get_api_token, get_max_request_size, get_request_per_seconds, update_max_request_size,
    update_request_per_seconds, update_user,
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
use crate::images::DockerHubImage;
use crate::images::Image;
use crate::renew_ssl_cert::handle_update_ssl_cert;
use crate::secrets;
use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use serde::{Deserialize, Serialize};

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
    // Brief read lock for access check + ready check
    {
        let stack = STACK.read().await;
        if !access(&cmd, &stack, user_id) {
            return Err(anyhow!("access denied"));
        }
        if !stack.ready && !cmd.can_run_before_ready() {
            return Err(anyhow!("cant run this command yet..."));
        }
    } // read lock dropped

    log::info!("=> CMD: {:?}", cmd);

    let ret: String = match cmd {
        Cmd::Swarm(c) => handle_swarm_cmd(proj, c, tag, docker, user_id).await?,
        Cmd::Relay(c) => handle_relay_cmd(proj, c, tag).await?,
        Cmd::Bitcoind(c) => handle_bitcoind_cmd(c, tag).await?,
        Cmd::Lnd(c) => handle_lnd_cmd(c, tag).await?,
        Cmd::Cln(c) => handle_cln_cmd(c, tag).await?,
        Cmd::Proxy(c) => handle_proxy_cmd(c, tag).await?,
        Cmd::Hsmd(c) => handle_hsmd_cmd(c, tag).await?,
    };

    Ok(ret)
}

// ── Swarm commands ──────────────────────────────────────────────────────────

async fn handle_swarm_cmd(
    proj: &str,
    c: SwarmCmd,
    _tag: &str,
    docker: &Docker,
    user_id: &Option<u32>,
) -> Result<String> {
    match c {
        // ── Read-only commands ──────────────────────────────────────────

        SwarmCmd::GetConfig => {
            let stack = STACK.read().await;
            let res = stack.remove_tokens();
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::GetSignedInUserDetails => {
            log::info!("Get Signed In Users details");
            if user_id.is_none() {
                return Ok("invalid user".to_string());
            }
            let uid = user_id.unwrap();
            let stack = STACK.read().await;
            match stack.users.iter().find(|user| user.id == uid) {
                Some(user) => {
                    let modified_user = User {
                        pass_hash: "".to_string(),
                        username: user.username.clone(),
                        id: user.id,
                        pubkey: user.pubkey.clone(),
                        role: user.role.clone(),
                    };
                    Ok(serde_json::to_string(&modified_user)?)
                }
                None => Ok("invalid user".to_string()),
            }
        }

        SwarmCmd::GetLightningPeers => {
            log::info!("Get all lightning peers");
            let stack = STACK.read().await;
            let res = &stack.lightning_peers;
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::GetNeo4jPassword => {
            log::info!("Get Neo4j Password");
            let stack = STACK.read().await;
            let res = get_neo4j_password(&stack.nodes);
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::GetBotToken => {
            log::info!("Get Bot Token");
            let stack = STACK.read().await;
            let res = get_bot_token(&stack.nodes);
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::GetBoltwallRequestPerSeconds => {
            log::info!("Get Boltwall Request Per Seconds");
            let stack = STACK.read().await;
            let boltwall = find_boltwall(&stack.nodes)?;
            let res = get_request_per_seconds(&boltwall);
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::GetBoltwallMaxRequestLimit => {
            log::info!("Get Boltwall Max Request Limit");
            let stack = STACK.read().await;
            let boltwall = find_boltwall(&stack.nodes)?;
            let res = get_max_request_size(&boltwall);
            Ok(serde_json::to_string(&res)?)
        }

        // ── Write commands (stack mutation + save) ──────────────────────

        SwarmCmd::SetGlobalMemLimit(gbm) => {
            let mut stack = STACK.write().await;
            stack.global_mem_limit = Some(gbm);
            config::put_config_file(proj, &stack).await;
            Ok(crate::config::set_global_mem_limit(gbm)?)
        }

        SwarmCmd::AddLightningPeer(info) => {
            log::info!("Add new lightning peer");
            let mut stack = STACK.write().await;
            let mut must_save = false;
            let res = add_new_lightning_peer(&mut stack, info, &mut must_save);
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::UpdateLightningPeer(info) => {
            log::info!("Update Lightning peer");
            let mut stack = STACK.write().await;
            let mut must_save = false;
            let res = update_lightning_peer(&mut stack, info, &mut must_save);
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::UpdateAdminPubkey(details) => {
            let mut stack = STACK.write().await;
            match stack.users.iter().position(|u| u.id == details.user_id) {
                Some(ui) => {
                    stack.users[ui].pubkey = Some(details.pubkey.to_string());
                    config::put_config_file(proj, &stack).await;
                    let mut hm = HashMap::new();
                    hm.insert("success", true);
                    Ok(serde_json::to_string(&hm)?)
                }
                None => Ok("invalid user".to_string()),
            }
        }

        SwarmCmd::UpdateNeo4jConfig(body) => {
            log::info!("Update Neo4j config ===> {:?}", body);
            let mut stack = STACK.write().await;
            let mut updated = false;
            for node in stack.nodes.iter_mut() {
                if let Node::Internal(Image::Neo4j(ref mut neo)) = node {
                    if let Some(v) = body.heap_initial_gb {
                        neo.heap_initial_gb = Some(v);
                    }
                    if let Some(v) = body.heap_max_gb {
                        neo.heap_max_gb = Some(v);
                    }
                    if let Some(v) = body.pagecache_gb {
                        neo.pagecache_gb = Some(v);
                    }
                    if let Some(v) = body.tx_total_gb {
                        neo.tx_total_gb = Some(v);
                    }
                    if let Some(v) = body.tx_max_gb {
                        neo.tx_max_gb = Some(v);
                    }
                    if let Some(v) = body.checkpoint_iops {
                        neo.checkpoint_iops = Some(v);
                    }
                    updated = true;
                }
            }
            let res = if updated {
                config::put_config_file(proj, &stack).await;
                SwarmResponse {
                    success: true,
                    message: "neo4j config updated; restart neo4j container to apply changes"
                        .to_string(),
                    data: None,
                }
            } else {
                SwarmResponse {
                    success: false,
                    message: "neo4j image not found in stack".to_string(),
                    data: None,
                }
            };
            Ok(serde_json::to_string(&res)?)
        }

        // ── Login: read lock, clone user data, drop, bcrypt outside ─────

        SwarmCmd::Login(ld) => {
            let user_data = {
                let stack = STACK.read().await;
                stack
                    .users
                    .iter()
                    .find(|u| u.username == ld.username)
                    .cloned()
            };
            match user_data {
                Some(user) => {
                    if !bcrypt::verify(&ld.password, &user.pass_hash)? {
                        Ok("".to_string())
                    } else {
                        let mut hm = HashMap::new();
                        hm.insert("token", auth::make_jwt(user.id)?);
                        Ok(serde_json::to_string(&hm)?)
                    }
                }
                None => Ok("".to_string()),
            }
        }

        // ── ChangePassword: read lock for old hash, bcrypt outside, write lock to save ──

        SwarmCmd::ChangePassword(cp) => {
            let old_hash = {
                let stack = STACK.read().await;
                stack
                    .users
                    .iter()
                    .find(|u| u.id == cp.user_id)
                    .map(|u| u.pass_hash.clone())
            };
            match old_hash {
                Some(hash) => {
                    if bcrypt::verify(&cp.old_pass, &hash)? {
                        let new_hash = bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                        let mut stack = STACK.write().await;
                        if let Some(ui) = stack.users.iter().position(|u| u.id == cp.user_id) {
                            stack.users[ui].pass_hash = new_hash;
                            config::put_config_file(proj, &stack).await;
                        }
                        let mut hm = HashMap::new();
                        hm.insert("success", true);
                        Ok(serde_json::to_string(&hm)?)
                    } else {
                        Ok("".to_string())
                    }
                }
                None => Ok("".to_string()),
            }
        }

        SwarmCmd::ChangeAdmin(cp) => {
            let old_hash = {
                let stack = STACK.read().await;
                stack
                    .users
                    .iter()
                    .find(|u| u.id == cp.user_id)
                    .map(|u| u.pass_hash.clone())
            };
            match old_hash {
                Some(hash) => {
                    if bcrypt::verify(&cp.old_pass, &hash)? {
                        let new_hash = bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                        let mut stack = STACK.write().await;
                        if let Some(ui) = stack.users.iter().position(|u| u.id == cp.user_id) {
                            stack.users[ui].pass_hash = new_hash;
                            stack.users[ui].username = cp.email.clone();
                            config::put_config_file(proj, &stack).await;
                        }
                        let mut hm = HashMap::new();
                        hm.insert("success", true);
                        Ok(serde_json::to_string(&hm)?)
                    } else {
                        Ok("".to_string())
                    }
                }
                None => Ok("".to_string()),
            }
        }

        // ── Commands needing boltwall info then HTTP (read lock, drop, HTTP) ──

        SwarmCmd::AddBoltwallAdminPubkey(admin) => {
            log::info!(
                "AddBoltwallAdminPubkey ->pubkey {}, name {:?}",
                admin.pubkey,
                admin.name
            );
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let name = admin.name.unwrap_or("".to_string());
            let response =
                crate::conn::boltwall::add_admin_pubkey(&boltwall, &admin.pubkey, &name).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::GetBoltwallSuperAdmin => {
            log::info!("GetBoltwallSuperAdmin");
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::get_super_admin(&boltwall).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::ListAdmins => {
            log::info!("ListAdmins ==> ");
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::list_admins(&boltwall).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::ListPaidEndpoint => {
            log::info!("ListPaidEndpoint ===> ");
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::list_paid_endpoint(&boltwall).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::UpdatePaidEndpoint(details) => {
            log::info!(
                "UpdatePaidEndpoint -> Status:{} ID:{}",
                details.status,
                details.id
            );
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::update_paid_endpoint(
                &boltwall,
                details.id,
                details.status,
            )
            .await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::UpdateEndpointPrice(details) => {
            log::info!(
                "UpdateEndpointPrice -> ID:{} Price:{}",
                details.id,
                details.price
            );
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::update_endpoint_price(
                &boltwall,
                details.id,
                details.price,
            )
            .await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::UpdateBoltwallAccessibility(is_public) => {
            log::info!("UpdateBoltwallAccessibility -> Status:{} ", is_public);
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response =
                crate::conn::boltwall::update_boltwall_accessibility(&boltwall, is_public).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::GetBoltwallAccessibility => {
            log::info!("Get Boltwall Accessibility ===>");
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::get_boltwall_accessibility(&boltwall).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::GetFeatureFlags => {
            log::info!("Get Boltwall Feature Flags ===>");
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::get_feature_flags(&boltwall).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::GetSecondBrainAboutDetails => {
            log::info!("Get Second Brain About Details ===>");
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response =
                crate::conn::boltwall::get_second_brain_about_details(&boltwall).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::UpdateSecondBrainAbout(about) => {
            log::info!("Update Second Brain Title: {:?}", about.title);
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response =
                crate::conn::boltwall::update_second_brain_about(&boltwall, about).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::UpdateFeatureFlags(body) => {
            log::info!("Update Feature Flags ===> {:?}", body);
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = crate::conn::boltwall::update_feature_flags(&boltwall, body).await?;
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::GetApiToken => {
            log::info!("Get API TOKEN");
            let boltwall = {
                let stack = STACK.read().await;
                find_boltwall(&stack.nodes)?
            };
            let response = get_api_token(&boltwall).await?;
            Ok(serde_json::to_string(&response)?)
        }

        // ── Commands needing boltwall + stack mutation (write lock for the whole op) ──

        SwarmCmd::AddBoltwallUser(user) => {
            log::info!(
                "AddBoltwallUser -> pubkey {}-> role {} -> name {:?} ",
                user.pubkey,
                user.role,
                user.name
            );
            let mut stack = STACK.write().await;
            let boltwall = find_boltwall(&stack.nodes)?;
            let name = user.name.unwrap_or("".to_string());
            let mut must_save = false;
            let response = crate::conn::boltwall::add_user(
                &boltwall,
                &user.pubkey,
                user.role,
                name,
                &mut stack,
                &mut must_save,
            )
            .await?;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::DeleteSubAdmin(apk) => {
            log::info!("DeleteSubAdmin -> {}", apk);
            let mut stack = STACK.write().await;
            let boltwall = find_boltwall(&stack.nodes)?;
            let mut must_save = false;
            let response = crate::conn::boltwall::delete_sub_admin(
                &boltwall,
                &apk,
                &mut stack,
                &mut must_save,
            )
            .await?;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&response)?)
        }

        SwarmCmd::UpdateUser(body) => {
            log::info!("Update users details ===> {:?}", body);
            let mut stack = STACK.write().await;
            let boltwall = find_boltwall(&stack.nodes)?;
            let mut must_save = false;
            let response = update_user(
                &boltwall,
                body.pubkey,
                body.name,
                body.id,
                body.role,
                &mut stack,
                &mut must_save,
            )
            .await?;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&response)?)
        }

        // ── Docker commands ─────────────────────────────────────────────

        SwarmCmd::StartContainer(id) => {
            log::info!("StartContainer -> {}", id);
            let res = start_container(docker, &id).await?;
            // extra startup steps such as LND unlock
            let img = {
                let stack = STACK.read().await;
                builder::find_image_by_hostname(&stack.nodes, &id)?
            };
            if let Err(e) = img.post_startup(proj, docker).await {
                log::warn!("{:?}", e);
            }
            // need to recreate client here?
            let clients = CLIENTS.read().await;
            img.post_client(&clients).await?;
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::StopContainer(id) => {
            log::info!("StopContainer -> {}", id);
            let res = stop_container(docker, &id).await?;
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::RestartContainer(id) => {
            log::info!("RestartContainer -> {}", id);
            let stack = STACK.write().await;
            let mut clients = CLIENTS.write().await;
            let res = restart_node_container(docker, &id, &stack, &mut clients, proj).await?;
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::AddNode(node) => {
            log::info!("AddNode -> {:?}", node);
            // add a node via docker
            Ok(serde_json::to_string(&())?)
        }

        // ── UpdateNode: write lock on both STACK and CLIENTS ────────────

        SwarmCmd::UpdateNode(un) => {
            log::info!("UpdateNode -> {}", un.id);
            let mut stack = STACK.write().await;
            let mut clients = CLIENTS.write().await;
            for node in stack.nodes.iter_mut() {
                if node.name() == un.id {
                    let _ = node.set_version(&un.version)?;
                }
            }
            builder::update_node_and_make_client(proj, docker, &un.id, &stack, &mut clients)
                .await?;
            config::put_config_file(proj, &stack).await;
            Ok(serde_json::to_string("{}")?)
        }

        // ── No lock needed ──────────────────────────────────────────────

        SwarmCmd::ListContainers => {
            let containers = list_containers(docker).await?;
            Ok(serde_json::to_string(&containers)?)
        }

        SwarmCmd::GetStatistics(container_name) => {
            let docker = dockr();
            println!("Calling GetStatistics with {:?}", &container_name);
            let containers = get_container_statistics(&docker, container_name).await?;
            println!("GetStatistics Called");
            Ok(serde_json::to_string(&containers)?)
        }

        SwarmCmd::GetContainerLogs(container_name) => {
            let logs = container_logs(docker, &container_name).await;
            Ok(serde_json::to_string(&logs)?)
        }

        SwarmCmd::GetEnv(id) => {
            log::info!("Get {} env variables", id);
            let res = get_env_variables_by_container_name(docker, &id).await;
            Ok(serde_json::to_string(&res)?)
        }

        // ── ListVersions: read lock for repo info, drop, HTTP ───────────

        SwarmCmd::ListVersions(req) => {
            #[derive(Serialize, Deserialize, Debug, Clone)]
            struct ListVersionsResult {
                org: String,
                repo: String,
                images: String,
            }
            let img = {
                let stack = STACK.read().await;
                stack
                    .nodes
                    .iter()
                    .find(|n| n.name() == req.name)
                    .context(format!("cant find node {}", &req.name))?
                    .as_internal()?
                    .repo()
            };
            let url = format!(
                "https://hub.docker.com/v2/namespaces/{}/repositories/{}/tags?page={}",
                img.org, img.repo, req.page
            );
            let body = reqwest::get(url).await?.text().await?;
            Ok(serde_json::to_string(&ListVersionsResult {
                org: img.org.clone(),
                repo: img.repo.clone(),
                images: body,
            })?)
        }

        // ── Stack mutation + save ───────────────────────────────────────

        SwarmCmd::ChangeUserPasswordBySuperAdmin(info) => {
            log::info!("Change user password from superadmin");
            let mut stack = STACK.write().await;
            let mut must_save = false;
            let res = change_swarm_user_password_by_user_admin(
                &mut stack,
                user_id.clone(),
                info,
                &mut must_save,
            )
            .await;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::SignUpAdminPubkey(body) => {
            log::info!("Signup Admin Pubkey ===> {:?}", body);
            let mut stack = STACK.write().await;
            let mut must_save = false;
            let response = sign_up_admin_pubkey(body, &mut must_save, &mut stack).await?;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&response)?)
        }

        // ── UpdateBoltwallRequestPerSeconds/MaxRequestLimit: STACK + CLIENTS write ──

        SwarmCmd::UpdateBoltwallRequestPerSeconds(info) => {
            log::info!(
                "Update Boltwall Request per seconds to: {}",
                &info.request_per_seconds
            );
            let mut stack = STACK.write().await;
            let mut clients = CLIENTS.write().await;
            let mut must_save = false;
            let res = update_request_per_seconds(
                info.request_per_seconds,
                &mut stack,
                &mut clients,
                &mut must_save,
                docker,
                proj,
            )
            .await;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::UpdateBoltwallMaxRequestLimit(info) => {
            log::info!(
                "Update Boltwall Max Request Limit: {}",
                &info.max_request_limit
            );
            let mut stack = STACK.write().await;
            let mut clients = CLIENTS.write().await;
            let mut must_save = false;
            let res = update_max_request_size(
                &info.max_request_limit,
                &mut stack,
                &mut clients,
                &mut must_save,
                docker,
                proj,
            )
            .await;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        // ── UpdateEvn: STACK write ──────────────────────────────────────

        SwarmCmd::UpdateEvn(update_env) => {
            log::info!("Update env variables for {:#?}", update_env.id);
            let mut stack = STACK.write().await;
            let mut must_save = false;
            let res = update_env_variables(
                docker,
                &mut update_env.clone(),
                &mut stack,
                &mut must_save,
            )
            .await;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        // ── ChangeReservedSwarmToActive: STACK write ────────────────────

        SwarmCmd::ChangeReservedSwarmToActive(details) => {
            log::info!("About to update reserved swarm to active");
            let mut stack = STACK.write().await;
            let mut must_save = false;
            let res = handle_assign_reserved_swarm_to_active(
                docker,
                &details,
                user_id.clone(),
                &mut stack,
                &mut must_save,
            )
            .await;
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        // ── UpdateSslCert: STACK write ──────────────────────────────────

        SwarmCmd::UpdateSslCert => {
            let mut stack = STACK.write().await;
            let mut must_save = false;
            let res = match handle_update_ssl_cert(&mut stack, &mut must_save).await {
                Ok(_) => SwarmResponse {
                    success: true,
                    message: "ssl certificate updated successfully".to_string(),
                    data: None,
                },
                Err(err) => SwarmResponse {
                    success: false,
                    message: err.to_string(),
                    data: None,
                },
            };
            if must_save {
                config::put_config_file(proj, &stack).await;
            }
            Ok(serde_json::to_string(&res)?)
        }

        // ── UpdateSwarm: no lock needed (just HTTP) ─────────────────────

        SwarmCmd::UpdateSwarm => {
            log::info!("Updating Swarm ===>");
            let response = crate::conn::swarm::update_swarm().await?;
            Ok(serde_json::to_string(&response)?)
        }

        // ── GetBotBalance, CreateBotInvoice: STACK read ─────────────────

        SwarmCmd::GetBotBalance => {
            log::info!("Get Bot Balance");
            let stack = STACK.read().await;
            let res = get_bot_balance(&stack.nodes).await;
            Ok(serde_json::to_string(&res)?)
        }

        SwarmCmd::CreateBotInvoice(body) => {
            log::info!("Create Bot Invoice");
            let stack = STACK.read().await;
            let res = create_bot_invoice(&stack.nodes, body.amt_msat).await;
            Ok(serde_json::to_string(&res)?)
        }

        // ── GetAllImageActualVersion: STACK read for nodes, then Docker ─

        SwarmCmd::GetAllImageActualVersion => {
            log::info!("Get all Image actual version");
            let nodes = {
                let stack = STACK.read().await;
                stack.nodes.clone()
            };
            let image_versions = get_image_actual_version(&nodes).await?;
            Ok(serde_json::to_string(&image_versions)?)
        }

        // ── No lock needed ──────────────────────────────────────────────

        SwarmCmd::GetImageDigest(image_name) => {
            let digest = get_image_digest(&image_name).await?;
            Ok(serde_json::to_string(&digest)?)
        }

        SwarmCmd::GetDockerImageTags(image_details) => {
            log::info!("Get Docker Image Tags ===> {:?}", image_details);
            let tags = get_image_tags(image_details).await?;
            Ok(serde_json::to_string(&tags)?)
        }
    }
}

// ── Relay commands ──────────────────────────────────────────────────────────

async fn handle_relay_cmd(proj: &str, c: RelayCmd, tag: &str) -> Result<String> {
    let client = {
        let clients = CLIENTS.read().await;
        clients
            .relay
            .get(tag)
            .context("no relay client")?
            .clone()
    };
    match c {
        RelayCmd::AddUser(u) => Ok(client.add_user(u.initial_sats).await?.to_string()?),
        RelayCmd::ListUsers => Ok(client.list_users().await?.to_string()?),
        RelayCmd::GetChats => Ok(client.get_chats().await?.to_string()?),
        RelayCmd::AddDefaultTribe(t) => Ok(client.add_default_tribe(t.id).await?.to_string()?),
        RelayCmd::RemoveDefaultTribe(t) => {
            Ok(client.remove_default_tribe(t.id).await?.to_string()?)
        }
        RelayCmd::GetToken => {
            let secs = secrets::load_secrets(proj).await;
            let token = secs.get(tag).context("no relay token")?;
            let mut hm = HashMap::new();
            hm.insert("token", base64::encode(token));
            Ok(serde_json::to_string(&hm)?)
        }
        RelayCmd::GetBalance => Ok(client.get_balance().await?.to_string()?),
    }
}

// ── Bitcoind commands ───────────────────────────────────────────────────────

async fn handle_bitcoind_cmd(c: BitcoindCmd, tag: &str) -> Result<String> {
    let client = {
        let clients = CLIENTS.read().await;
        clients
            .bitcoind
            .get(tag)
            .context("no bitcoind client")?
            .clone()
    };
    match c {
        BitcoindCmd::GetInfo => {
            let info = client.get_info()?;
            Ok(serde_json::to_string(&info)?)
        }
        BitcoindCmd::TestMine(tm) => {
            let res = client.test_mine(tm.blocks, tm.address)?;
            Ok(serde_json::to_string(&res)?)
        }
        BitcoindCmd::GetBalance => {
            let res = client.get_wallet_balance()?;
            Ok(serde_json::to_string(&res)?)
        }
    }
}

// ── LND commands ────────────────────────────────────────────────────────────

async fn handle_lnd_cmd(c: LndCmd, tag: &str) -> Result<String> {
    let lnd_arc = {
        let clients = CLIENTS.read().await;
        clients.lnd.get(tag).context("no lnd client")?.clone()
    };
    let mut client = lnd_arc.lock().await;
    match c {
        LndCmd::GetInfo => {
            let info = client.get_info().await?;
            Ok(serde_json::to_string(&info)?)
        }
        LndCmd::ListChannels => {
            let channel_list = client.list_channels().await?;
            Ok(serde_json::to_string(&channel_list.channels)?)
        }
        LndCmd::AddPeer(peer) => {
            if let Some(alias) = peer.alias.clone() {
                {
                    let mut stack = STACK.write().await;
                    let mut must_save = false;
                    add_new_lightning_peer(
                        &mut stack,
                        LightningPeer {
                            pubkey: peer.pubkey.clone(),
                            alias,
                        },
                        &mut must_save,
                    );
                    // Note: no proj available here to save config inline;
                    // the must_save flag is consumed but config save would
                    // need the proj param which is not passed to handle_lnd_cmd.
                    // For now we skip persisting here — the peer is saved in memory.
                }
                let result = client.add_peer(peer).await?;
                Ok(serde_json::to_string(&result)?)
            } else {
                let result = client.add_peer(peer).await?;
                Ok(serde_json::to_string(&result)?)
            }
        }
        LndCmd::ListPeers => {
            let result = client.list_peers().await?;
            Ok(serde_json::to_string(&result)?)
        }
        LndCmd::AddChannel(channel) => {
            let channel = client.create_channel(channel).await?;
            Ok(serde_json::to_string(&channel)?)
        }
        LndCmd::NewAddress => {
            let address = client.new_address().await?;
            Ok(serde_json::to_string(&address.address)?)
        }
        LndCmd::GetBalance => {
            let bal = client.get_balance().await?;
            Ok(serde_json::to_string(&bal)?)
        }
        LndCmd::AddInvoice(invoice) => {
            let invoice = client.add_invoice(invoice).await?;
            Ok(serde_json::to_string(&invoice)?)
        }
        LndCmd::PayInvoice(invoice) => {
            let invoice = client.pay_invoice(invoice).await?;
            Ok(serde_json::to_string(&invoice)?)
        }
        LndCmd::PayKeysend(keysend) => {
            let invoice = client.pay_keysend(keysend).await?;
            Ok(serde_json::to_string(&invoice)?)
        }
        LndCmd::ListPayments => {
            let payments = client.list_payments().await?;
            Ok(serde_json::to_string(&payments)?)
        }
        LndCmd::ListInvoices => {
            let invoices = client.list_invoices().await?;
            Ok(serde_json::to_string(&invoices)?)
        }
        LndCmd::ListPendingChannels => {
            let pending_channel_list = client.list_pending_channels().await?;
            Ok(serde_json::to_string(
                &pending_channel_list.pending_open_channels,
            )?)
        }
    }
}

// ── CLN commands ────────────────────────────────────────────────────────────

async fn handle_cln_cmd(c: ClnCmd, tag: &str) -> Result<String> {
    let mut client = {
        let clients = CLIENTS.read().await;
        clients.cln.get(tag).context("no cln client")?.clone()
    };
    match c {
        ClnCmd::GetInfo => {
            let info = client.get_info().await?;
            Ok(serde_json::to_string(&info)?)
        }
        ClnCmd::ListPeers => {
            let info = client.list_peers().await?;
            Ok(serde_json::to_string(&info)?)
        }
        ClnCmd::ListPeerChannels => {
            let info = client.list_peer_channels(None).await?;
            Ok(serde_json::to_string(&info)?)
        }
        ClnCmd::ListFunds => {
            let funds = client.list_funds().await?;
            Ok(serde_json::to_string(&funds)?)
        }
        ClnCmd::NewAddress => {
            let address = client.new_addr().await?;
            Ok(serde_json::to_string(
                &address.bech32.unwrap_or("".to_string()),
            )?)
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
                {
                    let mut stack = STACK.write().await;
                    let mut must_save = false;
                    add_new_lightning_peer(
                        &mut stack,
                        LightningPeer {
                            alias,
                            pubkey: peer.pubkey.clone(),
                        },
                        &mut must_save,
                    );
                    // Same note as LND AddPeer: no proj param to persist config
                }
                // Re-clone client since we dropped and re-acquired
                let mut client = {
                    let clients = CLIENTS.read().await;
                    clients.cln.get(tag).context("no cln client")?.clone()
                };
                let result = client.connect_peer(&peer.pubkey, &host, port).await?;
                Ok(serde_json::to_string(&result)?)
            } else {
                let result = client.connect_peer(&peer.pubkey, &host, port).await?;
                Ok(serde_json::to_string(&result)?)
            }
        }
        ClnCmd::AddChannel(channel) => {
            let channel = client
                .fund_channel(
                    &channel.pubkey,
                    channel.amount.try_into()?,
                    Some(channel.satsperbyte.try_into()?),
                )
                .await?;
            Ok(serde_json::to_string(&channel)?)
        }
        ClnCmd::AddInvoice(i) => {
            let inv = client.create_invoice(i.amt_paid_sat as u64).await?;
            Ok(serde_json::to_string(&inv)?)
        }
        ClnCmd::PayInvoice(i) => {
            let paid = client.pay(&i.payment_request).await?;
            Ok(serde_json::to_string(&paid)?)
        }
        ClnCmd::PayKeysend(i) => {
            let paid = client
                .keysend(
                    &i.dest,
                    i.amt as u64,
                    i.route_hint,
                    i.maxfeepercent,
                    i.exemptfee,
                    None,
                )
                .await?;
            Ok(serde_json::to_string(&paid)?)
        }
        ClnCmd::CloseChannel(i) => {
            let closed = client.close(&i.id, &i.destination).await?;
            let mut hm = HashMap::new();
            hm.insert("type", closed.item_type.to_string());
            hm.insert("txid", hex::encode(closed.txid()));
            hm.insert("tx", hex::encode(closed.tx()));
            Ok(serde_json::to_string(&hm)?)
        }
        ClnCmd::ListInvoices(i) => match i {
            Some(hash) => {
                let invoices = client.list_invoices(hash.payment_hash).await?;
                Ok(serde_json::to_string(&invoices)?)
            }
            None => {
                let invoices = client.list_invoices(None).await?;
                Ok(serde_json::to_string(&invoices)?)
            }
        },
        ClnCmd::ListPays(i) => match i {
            Some(hash) => {
                let pays = client.list_pays(hash.payment_hash).await?;
                Ok(serde_json::to_string(&pays)?)
            }
            None => {
                let pays = client.list_pays(None).await?;
                Ok(serde_json::to_string(&pays)?)
            }
        },
    }
}

// ── Proxy commands ──────────────────────────────────────────────────────────

async fn handle_proxy_cmd(c: ProxyCmd, tag: &str) -> Result<String> {
    let client = {
        let clients = CLIENTS.read().await;
        clients
            .proxy
            .get(tag)
            .context("no proxy client")?
            .clone()
    };
    match c {
        ProxyCmd::GetBalance => {
            let balance = client.get_balance().await?;
            Ok(serde_json::to_string(&balance)?)
        }
    }
}

// ── Hsmd commands ───────────────────────────────────────────────────────────

async fn handle_hsmd_cmd(c: HsmdCmd, tag: &str) -> Result<String> {
    let client = {
        let clients = CLIENTS.read().await;
        clients
            .hsmd
            .get(tag)
            .context("no cln for hsmd client")?
            .clone()
    };
    match c {
        HsmdCmd::GetClients => {
            let clients = client.get_clients().await?;
            Ok(serde_json::to_string(&clients)?)
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

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

// ── Hydrate functions ───────────────────────────────────────────────────────

pub async fn hydrate(mut stack: Stack, clients: Clients) {
    stack.ready = true;
    {
        let mut s = STACK.write().await;
        *s = stack;
    }
    {
        let mut c = CLIENTS.write().await;
        *c = clients;
    }
}

pub async fn hydrate_stack(stack: Stack) {
    let mut s = STACK.write().await;
    *s = stack;
}

pub async fn hydrate_clients(clients: Clients) {
    let mut c = CLIENTS.write().await;
    *c = clients;
    let mut s = STACK.write().await;
    s.ready = true;
}

pub fn fmt_err(err: &str) -> String {
    format!("{{\"stack_error\":\"{}\"}}", err)
}
