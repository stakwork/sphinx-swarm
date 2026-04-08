mod auth_token;
mod aws_util;
mod checker;
pub mod cloudwatch;
mod cmd;
mod ec2;
mod lightning_bots;
mod route53;
mod routes;
mod service;
mod state;
mod util;

use cmd::{AddSwarmResponse, SuperSwarmResponse};
use cmd::{Cmd, SwarmCmd};
use lightning_bots::{
    change_lightning_bot_label, create_invoice_lightning_bot, get_lightning_bots_details,
};
use sphinx_swarm::utils::getenv;
use state::RemoteStack;
use state::Super;
use util::{
    accessing_child_container_controller, add_new_swarm_details, add_new_swarm_from_child_swarm,
    get_aws_instance_types, get_child_swarm_config, get_child_swarm_containers,
    get_child_swarm_credentials, get_child_swarm_image_versions, get_config_aws_data,
    get_config_update_state, get_swarm_instance_type, update_swarm_child_password,
    validate_instance_type_update, do_instance_type_aws_update, apply_instance_type_update,
};

use crate::checker::swarm_checker;
use crate::cmd::SuperRestarterResponse;
use crate::service::anthropic_key::add::handle_add_anthropic_key;
use crate::service::anthropic_key::get::handle_get_anthropic_keys;
use crate::service::child_swarm::update_env::update_child_swarm_env;
use crate::service::child_swarm::update_public_ip::{
    apply_public_ip_update, get_public_ip_route53_info,
};
use crate::service::log_group_migration::migrate_log_group_tags;
use crate::service::ssl_cert::handle_renew_cert::{
    handle_get_ssl_cert_expiry, renew_cert, upload_cert_to_s3,
};
use crate::service::ssl_cert::renew_cert_cron::ssl_cert_renewal_cron;
use crate::service::super_admin_logs::get_super_admin_docker_logs;
use crate::service::swarm_reserver::setup_cron::swarm_reserver_cron;
use crate::service::swarm_reserver::utils::check_reserve_swarm_flag_set;
use crate::service::update_super_admin::update_super_admin;
use crate::route53::add_domain_name_to_route53;
use crate::util::create_swarm_ec2;
use anyhow::{anyhow, Context, Result};
use rocket::tokio;
use routes::launch_rocket;
use sphinx_swarm::config::Role;
use sphinx_swarm::utils;
use sphinx_swarm::{auth, events, logs};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[rocket::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    sphinx_swarm::utils::setup_logs();

    let project = "super";
    let s: state::Super = load_config_file(project).await.expect("YAML CONFIG FAIL");
    log::info!("SUPER!!! {:?}", s);

    sphinx_swarm::auth::set_jwt_key(&s.jwt_key);

    state::hydrate(s).await;

    // Tag all existing EC2 instances with their log_group
    // tokio::spawn(async move {
    //     migrate_log_group_tags().await;
    // });

    let log_txs = logs::new_log_chans();
    let log_txs = Arc::new(Mutex::new(log_txs));

    let cron_handler_res = swarm_checker().await;
    if let Err(e) = cron_handler_res {
        log::error!("CRON failed {:?}", e);
    }

    if check_reserve_swarm_flag_set() {
        let cron_handle_reserver_swarm = swarm_reserver_cron().await;
        if let Err(e) = cron_handle_reserver_swarm {
            log::error!("SWARM RESERVER CRON failed {:?}", e);
        }
    }

    let ssl_cert_res = ssl_cert_renewal_cron().await;
    if let Err(e) = ssl_cert_res {
        log::error!("CRON failed {:?}", e);
    }

    // launch rocket
    let port = std::env::var("ROCKET_PORT").unwrap_or("8000".to_string());
    log::info!("🚀 => http://localhost:{}", port);

    let event_tx = events::new_event_chan();

    let _r = launch_rocket(project.to_string(), log_txs, event_tx).await?;

    Ok(())
}

pub async fn load_config_file(project: &str) -> Result<Super> {
    let yaml_path = format!("vol/{}/config.yaml", project);
    let s = utils::load_yaml(&yaml_path, Default::default()).await?;
    Ok(s)
}

pub async fn put_config_file(project: &str, rs: &Super) {
    let path = format!("vol/{}/config.yaml", project);
    utils::put_yaml(&path, rs).await;
}

fn access(cmd: &Cmd, state: &Super, user_id: &Option<u32>) -> bool {
    // login needs no auth
    match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::Login(_) => return true,
            SwarmCmd::SetChildSwarm(info) => {
                //get x-super-token
                let token = getenv("SUPER_TOKEN").unwrap_or("".to_string());
                if token.is_empty() {
                    return false;
                }
                if token != info.token {
                    return false;
                }
                return true;
            }
            SwarmCmd::CreateNewEc2Instance(info) => {
                let token = getenv("SUPER_TOKEN").unwrap_or("".to_string());
                if info.token.is_some() && !token.is_empty() && token == info.token.clone().unwrap()
                {
                    return true;
                }
            }
            SwarmCmd::UpdateChildSwarmPublicIp(info) => {
                let token = getenv("SUPER_TOKEN").unwrap_or("".to_string());
                if info.token.is_some() && !token.is_empty() && token == info.token.clone().unwrap()
                {
                    return true;
                }
            }
            SwarmCmd::StopEc2Instance(info) => {
                let token = getenv("SUPER_TOKEN").unwrap_or("".to_string());
                if info.token.is_some() && !token.is_empty() && token == info.token.clone().unwrap()
                {
                    return true;
                }
            }
            _ => {}
        },
    }
    // user id required if not SwarmCmd::Login
    if user_id.is_none() {
        return false;
    }
    let user_id = user_id.unwrap();
    let user = state.users.iter().find(|u| u.id == user_id);
    // user required
    if user.is_none() {
        return false;
    }

    return match user.unwrap().role {
        Role::Super => true,
        Role::Admin => false,
        Role::SubAdmin => false,
    };
}

// tag is the service name
pub async fn super_handle(
    proj: &str,
    cmd: Cmd,
    _tag: &str,
    user_id: &Option<u32>,
) -> Result<String> {
    // Brief read lock for access check
    {
        let state = state::STATE.read().await;
        if !access(&cmd, &state, user_id) {
            return Err(anyhow!("access denied"));
        }
    } // read lock dropped

    let ret: String = match cmd {
        Cmd::Swarm(swarm_cmd) => match swarm_cmd {
            // =============================================================
            // Fast reads — brief read lock, return immediately
            // =============================================================
            SwarmCmd::GetConfig => {
                // Phase 1: AWS call with NO lock held
                let aws_data = get_config_aws_data().await?;
                // Phase 2: brief write lock to update state with AWS data
                let mut state = state::STATE.write().await;
                let res = get_config_update_state(&mut state, aws_data)?;
                put_config_file(proj, &state).await;
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetAwsInstanceTypes => {
                let res = get_aws_instance_types();
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetInstanceType(info) => {
                let state = state::STATE.read().await;
                let res = match get_swarm_instance_type(info, &state) {
                    Ok(result) => result,
                    Err(err) => SuperSwarmResponse {
                        success: false, message: err.to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetAnthropicKey => {
                let state = state::STATE.read().await;
                let res = handle_get_anthropic_keys(&state);
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetChildSwarmCredentials(req) => {
                let state = state::STATE.read().await;
                let res = get_child_swarm_credentials(req, &state);
                serde_json::to_string(&res)?
            }

            // =============================================================
            // Fast writes — brief write lock, mutate, save, return
            // =============================================================
            SwarmCmd::AddNewSwarm(swarm) => {
                let mut state = state::STATE.write().await;
                let swarm_detail = RemoteStack {
                    host: swarm.host, user: Some("".to_string()),
                    pass: Some("".to_string()), ec2: Some(swarm.instance),
                    note: Some(swarm.description), default_host: "".to_string(),
                    ec2_instance_id: "".to_string(),
                    public_ip_address: Some("".to_string()),
                    private_ip_address: Some("".to_string()),
                    id: None, deleted: Some(false), route53_domain_names: None,
                    owner_pubkey: None, workspace_type: None, cln_pubkey: None,
                };
                let mut must_save = false;
                let hm = add_new_swarm_details(&mut state, swarm_detail, &mut must_save);
                if must_save { put_config_file(proj, &state).await; }
                serde_json::to_string(&hm)?
            }
            SwarmCmd::UpdateSwarm(swarm) => {
                let mut state = state::STATE.write().await;
                let hm = match state.stacks.iter().position(|u| u.host == swarm.id) {
                    Some(ui) => {
                        state.stacks[ui] = RemoteStack {
                            host: swarm.host, ec2: Some(swarm.instance),
                            note: Some(swarm.description),
                            user: state.stacks[ui].user.clone(),
                            pass: state.stacks[ui].pass.clone(),
                            default_host: state.stacks[ui].default_host.clone(),
                            ec2_instance_id: state.stacks[ui].ec2_instance_id.clone(),
                            public_ip_address: state.stacks[ui].public_ip_address.clone(),
                            private_ip_address: state.stacks[ui].private_ip_address.clone(),
                            id: state.stacks[ui].id.clone(),
                            deleted: state.stacks[ui].deleted.clone(),
                            route53_domain_names: state.stacks[ui].route53_domain_names.clone(),
                            owner_pubkey: state.stacks[ui].owner_pubkey.clone(),
                            workspace_type: state.stacks[ui].workspace_type.clone(),
                            cln_pubkey: state.stacks[ui].cln_pubkey.clone(),
                        };
                        put_config_file(proj, &state).await;
                        AddSwarmResponse { success: true, message: "Swarm updated successfully".to_string() }
                    }
                    None => AddSwarmResponse { success: false, message: "swarm does not exist".to_string() },
                };
                serde_json::to_string(&hm)?
            }
            SwarmCmd::DeleteSwarm(swarm) => {
                let mut state = state::STATE.write().await;
                let mut hm = HashMap::new();
                match state.delete_swarm_by_host(&swarm.host) {
                    Ok(()) => {
                        put_config_file(proj, &state).await;
                        hm.insert("success", "true".to_string());
                        hm.insert("message", "Swarm deleted successfully".to_string());
                    }
                    Err(msg) => {
                        hm.insert("message", msg.clone());
                        hm.insert("success", "false".to_string());
                    }
                }
                serde_json::to_string(&hm)?
            }
            SwarmCmd::SetChildSwarm(c) => {
                let mut state = state::STATE.write().await;
                let swarm_details = RemoteStack {
                    host: c.host, note: Some("".to_string()),
                    pass: Some(c.password), user: Some(c.username),
                    ec2: Some("".to_string()), default_host: c.default_host,
                    ec2_instance_id: "".to_string(),
                    public_ip_address: Some("".to_string()),
                    private_ip_address: Some("".to_string()),
                    id: c.id, deleted: Some(false), route53_domain_names: None,
                    owner_pubkey: None, workspace_type: None, cln_pubkey: None,
                };
                let mut must_save = false;
                let hm = add_new_swarm_from_child_swarm(&mut state, swarm_details, &mut must_save);
                if must_save { put_config_file(proj, &state).await; }
                serde_json::to_string(&hm)?
            }
            SwarmCmd::AddAnthropicKey(data) => {
                let mut state = state::STATE.write().await;
                let mut must_save = false;
                let res = handle_add_anthropic_key(&mut state, &mut must_save, data);
                if must_save { put_config_file(proj, &state).await; }
                serde_json::to_string(&res)?
            }
            SwarmCmd::ChangeLightningBotLabel(info) => {
                let mut state = state::STATE.write().await;
                let mut must_save = false;
                let res = change_lightning_bot_label(&mut state, &mut must_save, info).await;
                if must_save { put_config_file(proj, &state).await; }
                serde_json::to_string(&res)?
            }
            SwarmCmd::UpdateChildSwarmPublicIp(info) => {
                // Extract swarm info, determine if Route53 update is needed
                let route53_update = {
                    let state = state::STATE.read().await;
                    get_public_ip_route53_info(&state, &info)
                }; // read lock dropped

                // Do Route53 call (if needed) with NO lock held
                if let Some(ref r53) = route53_update {
                    if let Err(err) = add_domain_name_to_route53(r53.domains.clone(), &info.public_ip).await {
                        let message = format!("Failed to update Route53 record: {}", err);
                        log::error!("{}", message);
                        let res = SuperSwarmResponse {
                            success: false, message, data: None,
                        };
                        return Ok(serde_json::to_string(&res)?);
                    }
                }

                // Brief write lock to update state
                let mut state = state::STATE.write().await;
                let res = apply_public_ip_update(&mut state, &info);
                if res.success {
                    put_config_file(proj, &state).await;
                }
                serde_json::to_string(&res)?
            }

            // =============================================================
            // Login — read lock to get hash, drop, bcrypt outside lock
            // =============================================================
            SwarmCmd::Login(ld) => {
                let user_data = {
                    let state = state::STATE.read().await;
                    state.users.iter()
                        .find(|u| u.username == ld.username)
                        .cloned()
                }; // read lock dropped before bcrypt
                match user_data {
                    Some(user) => {
                        if !bcrypt::verify(&ld.password, &user.pass_hash)? {
                            "".to_string()
                        } else {
                            let mut hm = HashMap::new();
                            hm.insert("token", auth::make_jwt(user.id)?);
                            serde_json::to_string(&hm)?
                        }
                    }
                    None => "".to_string(),
                }
            }
            SwarmCmd::ChangePassword(cp) => {
                // Read old hash, drop lock, bcrypt verify+hash, then write
                let old_hash = {
                    let state = state::STATE.read().await;
                    state.users.iter()
                        .find(|u| u.id == cp.user_id)
                        .map(|u| u.pass_hash.clone())
                }; // read lock dropped
                match old_hash {
                    Some(hash) => {
                        if bcrypt::verify(&cp.old_pass, &hash)? {
                            let new_hash = bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                            let mut state = state::STATE.write().await;
                            if let Some(ui) = state.users.iter().position(|u| u.id == cp.user_id) {
                                state.users[ui].pass_hash = new_hash;
                                put_config_file(proj, &state).await;
                            }
                            let mut hm = HashMap::new();
                            hm.insert("success", true);
                            serde_json::to_string(&hm)?
                        } else {
                            "".to_string()
                        }
                    }
                    None => "".to_string(),
                }
            }

            // =============================================================
            // Slow I/O — read lock to get swarm info, DROP, then HTTP calls
            // =============================================================
            SwarmCmd::GetChildSwarmConfig(info) => {
                let swarm = {
                    let state = state::STATE.read().await;
                    state.find_swarm_by_host(&info.host, info.is_reserved)
                }; // lock dropped before HTTP
                let res = match swarm {
                    Some(s) => get_child_swarm_config(&s).await.unwrap_or_else(|e| SuperSwarmResponse {
                        success: false, message: e.to_string(), data: None,
                    }),
                    None => SuperSwarmResponse {
                        success: false, message: "Swarm does not exist".to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetChildSwarmContainers(info) => {
                let swarm = {
                    let state = state::STATE.read().await;
                    state.find_swarm_by_host(&info.host, info.is_reserved)
                };
                let res = match swarm {
                    Some(s) => get_child_swarm_containers(&s).await.unwrap_or_else(|e| SuperSwarmResponse {
                        success: false, message: e.to_string(), data: None,
                    }),
                    None => SuperSwarmResponse {
                        success: false, message: "Swarm does not exist".to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::StopChildSwarmContainers(info) => {
                let swarm = {
                    let state = state::STATE.read().await;
                    state.find_swarm_by_host(&info.host, info.is_reserved)
                };
                let res = match swarm {
                    Some(s) => {
                        use crate::util::access_child_swarm_containers;
                        access_child_swarm_containers(&s, info.nodes, "StopContainer").await.unwrap_or_else(|e| SuperSwarmResponse {
                            success: false, message: e.to_string(), data: None,
                        })
                    }
                    None => SuperSwarmResponse {
                        success: false, message: "Swarm does not exist".to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::StartChildSwarmContainers(info) => {
                let swarm = {
                    let state = state::STATE.read().await;
                    state.find_swarm_by_host(&info.host, info.is_reserved)
                };
                let res = match swarm {
                    Some(s) => {
                        use crate::util::access_child_swarm_containers;
                        access_child_swarm_containers(&s, info.nodes, "StartContainer").await.unwrap_or_else(|e| SuperSwarmResponse {
                            success: false, message: e.to_string(), data: None,
                        })
                    }
                    None => SuperSwarmResponse {
                        success: false, message: "Swarm does not exist".to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::RestartChildSwarmContainers(info) => {
                let swarm = {
                    let state = state::STATE.read().await;
                    state.find_swarm_by_host(&info.host, info.is_reserved)
                };
                let res = match swarm {
                    Some(s) => {
                        use crate::util::access_child_swarm_containers;
                        access_child_swarm_containers(&s, info.nodes, "RestartContainer").await.unwrap_or_else(|e| SuperSwarmResponse {
                            success: false, message: e.to_string(), data: None,
                        })
                    }
                    None => SuperSwarmResponse {
                        success: false, message: "Swarm does not exist".to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::UpdateChildSwarmContainers(info) => {
                let swarm = {
                    let state = state::STATE.read().await;
                    state.find_swarm_by_host(&info.host, info.is_reserved)
                };
                let res = match swarm {
                    Some(s) => {
                        use crate::util::access_child_swarm_containers;
                        access_child_swarm_containers(&s, info.nodes, "UpdateNode").await.unwrap_or_else(|e| SuperSwarmResponse {
                            success: false, message: e.to_string(), data: None,
                        })
                    }
                    None => SuperSwarmResponse {
                        success: false, message: "Swarm does not exist".to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetSwarmChildImageVersions(info) => {
                let swarm = {
                    let state = state::STATE.read().await;
                    state.find_swarm_by_host(&info.host, info.is_reserved)
                };
                let res = match swarm {
                    Some(s) => get_child_swarm_image_versions(&s).await.unwrap_or_else(|e| SuperSwarmResponse {
                        success: false, message: e.to_string(), data: None,
                    }),
                    None => SuperSwarmResponse {
                        success: false, message: "Swarm does not exist".to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::ChangeChildSwarmPassword(info) => {
                let state = state::STATE.read().await;
                let res = update_swarm_child_password(info, &state).await;
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetLightningBotsDetails => {
                let state = state::STATE.read().await;
                let res = get_lightning_bots_details(&state).await;
                serde_json::to_string(&res)?
            }
            SwarmCmd::CreateInvoiceForLightningBot(info) => {
                let state = state::STATE.read().await;
                let res = create_invoice_lightning_bot(&state, info).await;
                serde_json::to_string(&res)?
            }
            SwarmCmd::UpdateChildSwarmEnv(data) => {
                let state = state::STATE.read().await;
                let res = update_child_swarm_env(&state, data).await;
                serde_json::to_string(&res)?
            }

            // =============================================================
            // No lock needed — pure external I/O
            // =============================================================
            SwarmCmd::StopEc2Instance(info) => {
                let res = match crate::ec2::stop_ec2_instance_and_tag(&info.instance_id).await {
                    Ok(()) => SuperSwarmResponse {
                        success: true,
                        message: format!("Instance {} stopped successfully and tagged with DeletedOn", info.instance_id),
                        data: None,
                    },
                    Err(err) => SuperSwarmResponse {
                        success: false, message: err.to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetSuperAdminLogs => {
                let res = get_super_admin_docker_logs().await;
                serde_json::to_string(&res)?
            }
            SwarmCmd::RestartSuperAdmin => {
                let res = update_super_admin().await;
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetSslCertExpiry => {
                let res = handle_get_ssl_cert_expiry().await;
                serde_json::to_string(&res)?
            }
            SwarmCmd::RenewSslCert => {
                let res = match renew_cert().await {
                    Ok(data) => data,
                    Err(err) => SuperRestarterResponse {
                        ok: false, message: Some(err.to_string()), error: Some(err.to_string()),
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::UploadSSlCert => {
                let res = match upload_cert_to_s3().await {
                    Ok(data) => data,
                    Err(err) => SuperRestarterResponse {
                        ok: false, message: Some(err.to_string()), error: Some(err.to_string()),
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::GetEc2CpuUtilization(req) => {
                let res = match crate::cloudwatch::get_cpu_utilization(&req.instance_id).await {
                    Ok(Some(val)) => SuperSwarmResponse {
                        success: true, message: "ok".to_string(),
                        data: Some(serde_json::json!({ "cpu_percent": val })),
                    },
                    Ok(None) => SuperSwarmResponse {
                        success: true, message: "no data".to_string(), data: None,
                    },
                    Err(err) => SuperSwarmResponse {
                        success: false, message: err.to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }

            // =============================================================
            // CreateNewEc2Instance — THE BIG ONE (PM's bug)
            // Read lock briefly, drop, do AWS work (minutes), write lock to save
            // =============================================================
            SwarmCmd::CreateNewEc2Instance(info) => {
                // NOTE: create_swarm_ec2 still takes &mut Super and holds
                // the write lock during the entire EC2 provisioning (including
                // a 40s sleep). Splitting this function into phases requires
                // refactoring create_swarm_ec2 itself, which is complex due to
                // the handle_assign_reserved_swarm path. Read commands (GetConfig,
                // etc.) no longer block behind this since they use read locks,
                // but other write commands will wait.
                let mut state = state::STATE.write().await;
                let res = match create_swarm_ec2(&info, &mut state).await {
                    Ok(data) => {
                        put_config_file(proj, &state).await;
                        let display_name = info.name.as_ref().unwrap_or(&data.swarm_id).clone();
                        let parsed_data = serde_json::to_value(data)?;
                        SuperSwarmResponse {
                            success: true,
                            message: format!("{} was created successfully", display_name),
                            data: Some(parsed_data),
                        }
                    }
                    Err(err) => SuperSwarmResponse {
                        success: false, message: err.to_string(), data: None,
                    },
                };
                serde_json::to_string(&res)?
            }
            SwarmCmd::UpdateAwsInstanceType(info) => {
                // Phase 1: validate + extract under read lock
                let (ec2_id, domain_names) = {
                    let state = state::STATE.read().await;
                    validate_instance_type_update(&info, &state)?
                }; // read lock dropped

                // Phase 2: AWS calls with NO lock held
                do_instance_type_aws_update(&ec2_id, &info.instance_type, domain_names).await?;

                // Phase 3: brief write lock — re-finds by ec2_id (not stale index)
                let mut state = state::STATE.write().await;
                apply_instance_type_update(&mut state, &ec2_id, info.instance_type.clone());
                put_config_file(proj, &state).await;

                let res = SuperSwarmResponse {
                    success: true, message: "Instance updated successfully".to_string(), data: None,
                };
                serde_json::to_string(&res)?
            }
        },
    };

    Ok(ret)
}

pub fn fmt_err(err: &str) -> String {
    format!("{{\"stack_error\":\"{}\"}}", err)
}
