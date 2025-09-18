use std::collections::HashMap;

use crate::app_login::sign_up_admin_pubkey;
use crate::auth;
use crate::builder;
use crate::cmd::*;
use crate::config;
use crate::config::LightningPeer;
use crate::config::Role;
use crate::config::User;
use crate::config::{Clients, Node, Stack, State, STATE};
use crate::conn::boltwall::{
    get_api_token, get_max_request_size, get_request_per_seconds, update_max_request_size,
    update_request_per_seconds, update_user,
};
use crate::conn::swarm::add_new_lightning_peer;
use crate::conn::swarm::get_neo4j_password;
use crate::conn::swarm::handle_assign_reserved_swarm_to_active;
use crate::conn::swarm::update_lightning_peer;
use crate::conn::swarm::{
    change_swarm_user_password_by_user_admin, get_image_tags, update_env_variables,
};
use crate::dock::*;
use crate::images::DockerHubImage;
use crate::rocket_utils::CmdRequest;
use crate::secrets;
use anyhow::{anyhow, Context, Result};
use bollard::Docker;
use rocket::tokio;
use rocket::tokio::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

fn access(cmd: &Cmd, state: &State, user_id: &Option<u32>) -> bool {
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
    let user = state.stack.users.iter().find(|u| u.id == user_id);
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
    // conf can be mutated in place
    let mut state = config::STATE.lock().await;
    // println!("STACK {:?}", stack);

    if !access(&cmd, &state, user_id) {
        return Err(anyhow!("access denied"));
    }

    let mut must_save_stack = false;

    if !state.stack.ready {
        if !cmd.can_run_before_ready() {
            return Err(anyhow!("cant run this command yet..."));
        }
    }

    log::info!("=> CMD: {:?}", cmd);

    let ret: Option<String> = match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::GetConfig => {
                let res = &state.stack.remove_tokens();
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::StartContainer(id) => {
                log::info!("StartContainer -> {}", id);
                let res = start_container(docker, &id).await?;
                // extra startup steps such as LND unlock
                let img = builder::find_image_by_hostname(&state.stack.nodes, &id)?;
                if let Err(e) = img.post_startup(proj, docker).await {
                    log::warn!("{:?}", e);
                }
                // need to recreate client here?
                img.post_client(&state.clients).await?;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::StopContainer(id) => {
                log::info!("StopContainer -> {}", id);
                let res = stop_container(docker, &id).await?;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::RestartContainer(id) => {
                log::info!("RestartContainer -> {}", id);
                let res = restart_node_container(docker, &id, &mut state, proj).await?;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::AddNode(node) => {
                log::info!("AddNode -> {:?}", node);
                // add a node via docker
                None
            }
            SwarmCmd::UpdateNode(un) => {
                log::info!("UpdateNode -> {}", un.id);
                for node in state.stack.nodes.iter_mut() {
                    if node.name() == un.id {
                        let _ = node.set_version(&un.version)?;
                    }
                }
                builder::update_node_and_make_client(proj, &docker, &un.id, &mut state).await?;
                must_save_stack = true;
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
                let img = &state
                    .stack
                    .nodes
                    .iter()
                    .find(|n| n.name() == req.name)
                    .context(format!("cant find node {}", &req.name))?
                    .as_internal()?
                    .repo();
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
                match state.stack.users.iter().find(|u| u.username == ld.username) {
                    Some(user) => {
                        if !bcrypt::verify(&ld.password, &user.pass_hash)? {
                            Some("".to_string())
                        } else {
                            let mut hm = HashMap::new();
                            hm.insert("token", auth::make_jwt(user.id)?);
                            Some(serde_json::to_string(&hm)?)
                        }
                    }
                    None => Some("".to_string()),
                }
            }
            SwarmCmd::ChangePassword(cp) => {
                match state.stack.users.iter().position(|u| u.id == cp.user_id) {
                    Some(ui) => {
                        let old_pass_hash = &state.stack.users[ui].pass_hash;
                        if bcrypt::verify(&cp.old_pass, old_pass_hash)? {
                            state.stack.users[ui].pass_hash =
                                bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                            must_save_stack = true;
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
                match state.stack.users.iter().position(|u| u.id == cp.user_id) {
                    Some(ui) => {
                        let old_pass_hash = &state.stack.users[ui].pass_hash;
                        if bcrypt::verify(&cp.old_pass, old_pass_hash)? {
                            state.stack.users[ui].pass_hash =
                                bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                            state.stack.users[ui].username = cp.email.clone();
                            must_save_stack = true;
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
                log::info!(
                    "AddBoltwallAdminPubkey ->pubkey {}, name {:?}",
                    admin.pubkey,
                    admin.name
                );
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let name = admin.name.unwrap_or("".to_string());
                let response =
                    crate::conn::boltwall::add_admin_pubkey(&boltwall, &admin.pubkey, &name)
                        .await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::GetBoltwallSuperAdmin => {
                log::info!("GetBoltwallSuperAdmin");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::get_super_admin(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::AddBoltwallUser(user) => {
                log::info!(
                    "AddBoltwallUser -> pubkey {}-> role {} -> name {:?} ",
                    user.pubkey,
                    user.role,
                    user.name
                );
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let name = user.name.unwrap_or("".to_string());
                let response = crate::conn::boltwall::add_user(
                    &boltwall,
                    &user.pubkey,
                    user.role,
                    name,
                    &mut state,
                    &mut must_save_stack,
                )
                .await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::ListAdmins => {
                log::info!("ListAdmins ==> ");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::list_admins(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::DeleteSubAdmin(apk) => {
                log::info!("DeleteSubAdmin -> {}", apk);
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::delete_sub_admin(
                    &boltwall,
                    &apk,
                    &mut state,
                    &mut must_save_stack,
                )
                .await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::ListPaidEndpoint => {
                log::info!("ListPaidEndpoint ===> ");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::list_paid_endpoint(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateSwarm => {
                log::info!("Updating Swarm ===>");
                let response = crate::conn::swarm::update_swarm().await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdatePaidEndpoint(details) => {
                log::info!(
                    "UpdatePaidEndpoint -> Status:{} ID:{}",
                    details.status,
                    details.id
                );
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::update_paid_endpoint(
                    &boltwall,
                    details.id,
                    details.status,
                )
                .await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::UpdateBoltwallAccessibility(is_public) => {
                log::info!("UpdateBoltwallAccessibility -> Status:{} ", is_public);
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response =
                    crate::conn::boltwall::update_boltwall_accessibility(&boltwall, is_public)
                        .await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::GetBoltwallAccessibility => {
                log::info!("Get Boltwall Accessibility ===>");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::get_boltwall_accessibility(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }

            SwarmCmd::UpdateAdminPubkey(details) => {
                match state
                    .stack
                    .users
                    .iter()
                    .position(|u| u.id == details.user_id)
                {
                    Some(ui) => {
                        state.stack.users[ui].pubkey = Some(details.pubkey.to_string());
                        must_save_stack = true;
                        let mut hm = HashMap::new();
                        hm.insert("success", true);
                        Some(serde_json::to_string(&hm)?)
                    }
                    None => Some("invalid user".to_string()),
                }
            }

            SwarmCmd::GetFeatureFlags => {
                log::info!("Get Boltwall Feature Flags ===>");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::get_feature_flags(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }

            SwarmCmd::GetSecondBrainAboutDetails => {
                log::info!("Get Second Brain About Details ===>");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response =
                    crate::conn::boltwall::get_second_brain_about_details(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }

            SwarmCmd::UpdateSecondBrainAbout(about) => {
                log::info!("Update Second Brain Title: {:?}", about.title);
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response =
                    crate::conn::boltwall::update_second_brain_about(&boltwall, about).await?;
                Some(serde_json::to_string(&response)?)
            }

            SwarmCmd::UpdateFeatureFlags(body) => {
                log::info!("Update Feature Flags ===> {:?}", body);
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::update_feature_flags(&boltwall, body).await?;
                Some(serde_json::to_string(&response)?)
            }

            SwarmCmd::SignUpAdminPubkey(body) => {
                log::info!("Signup Admin Pubkey ===> {:?}", body);
                let response = sign_up_admin_pubkey(body, &mut must_save_stack, &mut state).await?;
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
                let image_versions = get_image_actual_version(&state.stack.nodes).await?;
                return Ok(serde_json::to_string(&image_versions)?);
            }
            SwarmCmd::UpdateUser(body) => {
                log::info!("Update users details ===> {:?}", body);
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = update_user(
                    &boltwall,
                    body.pubkey,
                    body.name,
                    body.id,
                    body.role,
                    &mut state,
                    &mut must_save_stack,
                )
                .await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::GetApiToken => {
                log::info!("Get API TOKEN");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = get_api_token(&boltwall).await?;
                return Ok(serde_json::to_string(&response)?);
            }
            SwarmCmd::SetGlobalMemLimit(gbm) => {
                state.stack.global_mem_limit = Some(gbm);
                must_save_stack = true;
                Some(crate::config::set_global_mem_limit(gbm)?)
            }
            SwarmCmd::GetSignedInUserDetails => {
                log::info!("Get Signed In Users details");
                if user_id.is_none() {
                    Some("invalid user".to_string());
                }
                let user_id = user_id.unwrap();
                match state.stack.users.iter().find(|user| user.id == user_id) {
                    Some(user) => {
                        let modified_user = User {
                            pass_hash: "".to_string(),
                            username: user.username.clone(),
                            id: user.id,
                            pubkey: user.pubkey.clone(),
                            role: user.role.clone(),
                        };
                        Some(serde_json::to_string(&modified_user)?)
                    }
                    None => Some("invalid user".to_string()),
                }
            }
            SwarmCmd::ChangeUserPasswordBySuperAdmin(info) => {
                log::info!("Change user password from superadmin");
                let res = change_swarm_user_password_by_user_admin(
                    &mut state,
                    user_id.clone(),
                    info,
                    &mut must_save_stack,
                )
                .await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetLightningPeers => {
                log::info!("Get all lightning peers");
                let res = &state.stack.lightning_peers;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::AddLightningPeer(info) => {
                log::info!("Add new lightning peer");
                let res = add_new_lightning_peer(&mut state, info, &mut must_save_stack);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateLightningPeer(info) => {
                log::info!("Update Lightning peer");
                let res = update_lightning_peer(&mut state, info, &mut must_save_stack);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetNeo4jPassword => {
                log::info!("Get Neo4j Password");
                let res = get_neo4j_password(&state.stack.nodes);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateBoltwallRequestPerSeconds(info) => {
                log::info!(
                    "Update Boltwall Request per seconds to: {}",
                    &info.request_per_seconds
                );
                // let mut boltwall = find_boltwall(&state.stack.nodes)?;
                let res = update_request_per_seconds(
                    info.request_per_seconds,
                    &mut state,
                    &mut must_save_stack,
                    docker,
                    proj,
                )
                .await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::GetBoltwallRequestPerSeconds => {
                log::info!("Get Boltwall Request Per Seconds");
                let boltwall = find_boltwall(&state.stack.nodes)?;
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
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let res = get_max_request_size(&boltwall);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateBoltwallMaxRequestLimit(info) => {
                log::info!(
                    "Update Boltwall Max Request Limit: {}",
                    &info.max_request_limit
                );
                let res = update_max_request_size(
                    &info.max_request_limit,
                    &mut state,
                    &mut must_save_stack,
                    docker,
                    proj,
                )
                .await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateEvn(update_env) => {
                log::info!("Update env variables for {:#?}", update_env.id);
                let res = update_env_variables(
                    &docker,
                    &mut update_env.clone(),
                    &mut state,
                    &mut must_save_stack,
                )
                .await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::ChangeReservedSwarmToActive(details) => {
                log::info!("About to update reserved swarm to active");
                let res = handle_assign_reserved_swarm_to_active(
                    &docker,
                    &details,
                    user_id.clone(),
                    &mut state,
                    &mut must_save_stack,
                )
                .await;
                Some(serde_json::to_string(&res)?)
            }
        },
        Cmd::Relay(c) => {
            let client = state.clients.relay.get(tag).context("no relay client")?;
            match c {
                RelayCmd::AddUser(u) => Some(client.add_user(u.initial_sats).await?.to_string()?),
                RelayCmd::ListUsers => Some(client.list_users().await?.to_string()?),
                RelayCmd::GetChats => Some(client.get_chats().await?.to_string()?),
                RelayCmd::AddDefaultTribe(t) => {
                    Some(client.add_default_tribe(t.id).await?.to_string()?)
                }
                RelayCmd::RemoveDefaultTribe(t) => {
                    Some(client.remove_default_tribe(t.id).await?.to_string()?)
                }
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
            let client = state
                .clients
                .bitcoind
                .get(tag)
                .context("no bitcoind client")?;

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
            let client = state.clients.lnd.get_mut(tag).context("no lnd client")?;
            match c {
                LndCmd::GetInfo => {
                    let info = client.get_info().await?;
                    Some(serde_json::to_string(&info)?)
                }
                LndCmd::ListChannels => {
                    let channel_list = client.list_channels().await?;
                    Some(serde_json::to_string(&channel_list.channels)?)
                }
                LndCmd::AddPeer(peer) => {
                    if let Some(alias) = peer.alias.clone() {
                        add_new_lightning_peer(
                            &mut state,
                            LightningPeer {
                                pubkey: peer.pubkey.clone(),
                                alias,
                            },
                            &mut must_save_stack,
                        );
                        let client = state.clients.lnd.get_mut(tag).context("no lnd client")?;
                        let result = client.add_peer(peer).await?;
                        Some(serde_json::to_string(&result)?)
                    } else {
                        let result = client.add_peer(peer).await?;
                        Some(serde_json::to_string(&result)?)
                    }
                }
                LndCmd::ListPeers => {
                    let result = client.list_peers().await?;
                    Some(serde_json::to_string(&result)?)
                }
                LndCmd::AddChannel(channel) => {
                    let channel = client.create_channel(channel).await?;
                    Some(serde_json::to_string(&channel)?)
                }
                LndCmd::NewAddress => {
                    let address = client.new_address().await?;
                    Some(serde_json::to_string(&address.address)?)
                }
                LndCmd::GetBalance => {
                    let bal = client.get_balance().await?;
                    Some(serde_json::to_string(&bal)?)
                }
                LndCmd::AddInvoice(invoice) => {
                    let invoice = client.add_invoice(invoice).await?;
                    Some(serde_json::to_string(&invoice)?)
                }
                LndCmd::PayInvoice(invoice) => {
                    let invoice = client.pay_invoice(invoice).await?;
                    Some(serde_json::to_string(&invoice)?)
                }
                LndCmd::PayKeysend(keysend) => {
                    let invoice = client.pay_keysend(keysend).await?;
                    Some(serde_json::to_string(&invoice)?)
                }
                LndCmd::ListPayments => {
                    let payments = client.list_payments().await?;
                    Some(serde_json::to_string(&payments)?)
                }
                LndCmd::ListInvoices => {
                    let invoices = client.list_invoices().await?;
                    Some(serde_json::to_string(&invoices)?)
                }
                LndCmd::ListPendingChannels => {
                    let pending_channel_list = client.list_pending_channels().await?;
                    Some(serde_json::to_string(
                        &pending_channel_list.pending_open_channels,
                    )?)
                }
            }
        }
        Cmd::Cln(c) => {
            let client = state.clients.cln.get_mut(tag).context("no cln client")?;
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
                    Some(serde_json::to_string(
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
                        add_new_lightning_peer(
                            &mut state,
                            LightningPeer {
                                alias,
                                pubkey: peer.pubkey.clone(),
                            },
                            &mut must_save_stack,
                        );
                        let client = state.clients.cln.get_mut(tag).context("no cln client")?;
                        let result = client.connect_peer(&peer.pubkey, &host, port).await?;
                        Some(serde_json::to_string(&result)?)
                    } else {
                        let result = client.connect_peer(&peer.pubkey, &host, port).await?;
                        Some(serde_json::to_string(&result)?)
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
            let client = state.clients.proxy.get(tag).context("no proxy client")?;
            match c {
                ProxyCmd::GetBalance => {
                    let balance = client.get_balance().await?;
                    Some(serde_json::to_string(&balance)?)
                }
            }
        }
        Cmd::Hsmd(c) => {
            let client = state
                .clients
                .hsmd
                .get_mut(tag)
                .context("no cln for hsmd client")?;
            match c {
                HsmdCmd::GetClients => {
                    let clients = client.get_clients().await?;
                    Some(serde_json::to_string(&clients)?)
                }
            }
        }
    };

    if must_save_stack {
        config::put_config_file(proj, &state.stack).await;
    }
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

pub async fn hydrate(mut stack: Stack, clients: Clients) {
    // set into the main state mutex
    stack.ready = true;
    let mut state = STATE.lock().await;
    *state = State { stack, clients };
}

pub async fn hydrate_stack(stack: Stack) {
    let mut state = STATE.lock().await;
    state.stack = stack
}

pub async fn hydrate_clients(clients: Clients) {
    let mut state = STATE.lock().await;
    state.clients = clients;
    state.stack.ready = true;
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
