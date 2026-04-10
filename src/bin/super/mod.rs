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
use state::{state_read, state_write};
use util::{
    accessing_child_container_controller, add_new_swarm_details, add_new_swarm_from_child_swarm,
    get_aws_instance_types, get_child_swarm_config, get_child_swarm_containers,
    get_child_swarm_credentials, get_child_swarm_image_versions, get_config,
    get_swarm_instance_type, update_aws_instance_type, update_swarm_child_password,
};

use crate::checker::swarm_checker;
use crate::cmd::SuperRestarterResponse;
use crate::service::anthropic_key::add::handle_add_anthropic_key;
use crate::service::anthropic_key::get::handle_get_anthropic_keys;
use crate::service::child_swarm::update_env::update_child_swarm_env;
use crate::service::child_swarm::update_public_ip::handle_update_child_swarm_public_ip;
#[allow(unused_imports)]
use crate::service::log_group_migration::migrate_log_group_tags;
use crate::service::ssl_cert::handle_renew_cert::{
    handle_get_ssl_cert_expiry, renew_cert, upload_cert_to_s3,
};
use crate::service::ssl_cert::renew_cert_cron::ssl_cert_renewal_cron;
use crate::service::super_admin_logs::get_super_admin_docker_logs;
use crate::service::swarm_reserver::nuke_warm_swarm::{
    nuke_all_warm_swarms, nuke_warm_swarm_by_host,
};
use crate::service::swarm_reserver::setup_cron::swarm_reserver_cron;
use crate::service::swarm_reserver::utils::check_reserve_swarm_flag_set;
use crate::service::update_super_admin::update_super_admin;
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
    // access check: brief read lock
    let allowed = state_read(|s| access(&cmd, s, user_id)).await;
    if !allowed {
        return Err(anyhow!("access denied"));
    }

    let ret: Option<String> = match cmd {
        Cmd::Swarm(swarm_cmd) => match swarm_cmd {
            // Pattern 5: read state, do AWS I/O, write result back
            SwarmCmd::GetConfig => {
                let res = get_config(proj).await?;
                Some(serde_json::to_string(&res)?)
            }
            // Login: read hash, verify outside lock
            SwarmCmd::Login(ld) => {
                let user_data = state_read(|s| {
                    s.users
                        .iter()
                        .find(|u| u.username == ld.username)
                        .map(|u| (u.id, u.pass_hash.clone()))
                })
                .await;
                match user_data {
                    Some((uid, pass_hash)) => {
                        if !bcrypt::verify(&ld.password, &pass_hash)? {
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
            // ChangePassword: read hash, verify+hash outside, write new hash
            SwarmCmd::ChangePassword(cp) => {
                let hash_data = state_read(|s| {
                    s.users
                        .iter()
                        .find(|u| u.id == cp.user_id)
                        .map(|u| u.pass_hash.clone())
                })
                .await;
                match hash_data {
                    Some(old_pass_hash) => {
                        if bcrypt::verify(&cp.old_pass, &old_pass_hash)? {
                            let new_hash = bcrypt::hash(cp.password, bcrypt::DEFAULT_COST)?;
                            let user_id = cp.user_id;
                            state_write(proj, |s| {
                                if let Some(u) = s.users.iter_mut().find(|u| u.id == user_id) {
                                    u.pass_hash = new_hash;
                                }
                            })
                            .await;
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
            // Pattern 2: Mutate and return
            SwarmCmd::AddNewSwarm(swarm) => {
                let swarm_detail = RemoteStack {
                    host: swarm.host,
                    user: Some("".to_string()),
                    pass: Some("".to_string()),
                    ec2: Some(swarm.instance),
                    note: Some(swarm.description),
                    default_host: "".to_string(),
                    ec2_instance_id: "".to_string(),
                    public_ip_address: Some("".to_string()),
                    private_ip_address: Some("".to_string()),
                    id: None,
                    deleted: Some(false),
                    route53_domain_names: None,
                    owner_pubkey: None,
                    workspace_type: None,
                    cln_pubkey: None,
                };
                let hm = state_write(proj, |s| add_new_swarm_details(s, swarm_detail)).await;
                Some(serde_json::to_string(&hm)?)
            }
            // Pattern 2: Mutate and return
            SwarmCmd::UpdateSwarm(swarm) => {
                let hm = state_write(proj, |s| {
                    match s.stacks.iter().position(|u| u.host == swarm.id) {
                        Some(ui) => {
                            s.stacks[ui] = RemoteStack {
                                host: swarm.host,
                                ec2: Some(swarm.instance),
                                note: Some(swarm.description),
                                user: s.stacks[ui].user.clone(),
                                pass: s.stacks[ui].pass.clone(),
                                default_host: s.stacks[ui].default_host.clone(),
                                ec2_instance_id: s.stacks[ui].ec2_instance_id.clone(),
                                public_ip_address: s.stacks[ui].public_ip_address.clone(),
                                private_ip_address: s.stacks[ui].private_ip_address.clone(),
                                id: s.stacks[ui].id.clone(),
                                deleted: s.stacks[ui].deleted.clone(),
                                route53_domain_names: s.stacks[ui].route53_domain_names.clone(),
                                owner_pubkey: s.stacks[ui].owner_pubkey.clone(),
                                workspace_type: s.stacks[ui].workspace_type.clone(),
                                cln_pubkey: s.stacks[ui].cln_pubkey.clone(),
                            };
                            AddSwarmResponse {
                                success: true,
                                message: "Swarm updated successfully".to_string(),
                            }
                        }
                        None => AddSwarmResponse {
                            success: false,
                            message: "swarm does not exist".to_string(),
                        },
                    }
                })
                .await;
                Some(serde_json::to_string(&hm)?)
            }
            // Pattern 2: Mutate and return
            SwarmCmd::DeleteSwarm(swarm) => {
                let (hm, domain_names_to_delete) = state_write(proj, |s| {
                    let mut hm = HashMap::new();
                    // Capture domain names before removing from state
                    let domain_names = s
                        .find_swarm_by_host(&swarm.host, None)
                        .and_then(|s| s.route53_domain_names)
                        .filter(|d| !d.is_empty());
                    match s.delete_swarm_by_host(&swarm.host) {
                        Ok(()) => {
                            hm.insert("success", "true".to_string());
                            hm.insert("message", "Swarm deleted successfully".to_string());
                        }
                        Err(msg) => {
                            hm.insert("message", msg.clone());
                            hm.insert("success", "false".to_string());
                        }
                    }
                    (hm, domain_names)
                })
                .await;
                // Best-effort Route53 cleanup after state is saved
                if let Some(domain_names) = domain_names_to_delete {
                    tokio::spawn(async move {
                        match route53::delete_multiple_route53_records(domain_names.clone()).await {
                            Ok(_) => log::info!(
                                "Deleted route53 records for deleted swarm: {:?}",
                                domain_names
                            ),
                            Err(err) => log::error!(
                                "Error deleting route53 records for swarm {:?}: {}",
                                domain_names,
                                err
                            ),
                        }
                    });
                }
                Some(serde_json::to_string(&hm)?)
            }
            // Pattern 2: Mutate and return
            SwarmCmd::SetChildSwarm(c) => {
                let swarm_details = RemoteStack {
                    host: c.host,
                    note: Some("".to_string()),
                    pass: Some(c.password),
                    user: Some(c.username),
                    ec2: Some("".to_string()),
                    default_host: c.default_host,
                    ec2_instance_id: "".to_string(),
                    public_ip_address: Some("".to_string()),
                    private_ip_address: Some("".to_string()),
                    id: c.id,
                    deleted: Some(false),
                    route53_domain_names: None,
                    owner_pubkey: None,
                    workspace_type: None,
                    cln_pubkey: None,
                };
                let hm =
                    state_write(proj, |s| add_new_swarm_from_child_swarm(s, swarm_details)).await;
                Some(serde_json::to_string(&hm)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::GetChildSwarmConfig(info) => {
                let swarm = state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved))
                    .await;
                let res = match swarm {
                    Some(swarm) => match get_child_swarm_config(&swarm).await {
                        Ok(result) => result,
                        Err(err) => SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        },
                    },
                    None => SuperSwarmResponse {
                        success: false,
                        message: "Swarm does not exist".to_string(),
                        data: None,
                    },
                };
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::GetChildSwarmContainers(info) => {
                let swarm = state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved))
                    .await;
                let res = match swarm {
                    Some(swarm) => match get_child_swarm_containers(&swarm).await {
                        Ok(result) => result,
                        Err(err) => SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        },
                    },
                    None => SuperSwarmResponse {
                        success: false,
                        message: "Swarm does not exist".to_string(),
                        data: None,
                    },
                };
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::StopChildSwarmContainers(info) => {
                let swarm =
                    state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved)).await;
                let res = accessing_child_container_controller(swarm, info.nodes, "StopContainer").await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::StartChildSwarmContainers(info) => {
                let swarm =
                    state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved)).await;
                let res = accessing_child_container_controller(swarm, info.nodes, "StartContainer").await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::RestartChildSwarmContainers(info) => {
                let swarm =
                    state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved)).await;
                let res =
                    accessing_child_container_controller(swarm, info.nodes, "RestartContainer")
                        .await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::UpdateChildSwarmContainers(info) => {
                let swarm =
                    state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved)).await;
                let res =
                    accessing_child_container_controller(swarm, info.nodes, "UpdateNode").await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed
            SwarmCmd::GetAwsInstanceTypes => {
                let res = get_aws_instance_types();
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 6: Claim resources, do I/O, write result (with rollback)
            SwarmCmd::CreateNewEc2Instance(info) => {
                let res = create_swarm_ec2(&info, proj).await;
                let res: SuperSwarmResponse = match res {
                    Ok(data) => {
                        let display_name = info.name.as_ref().unwrap_or(&data.swarm_id).clone();
                        let parsed_data = serde_json::to_value(data)?;
                        SuperSwarmResponse {
                            success: true,
                            message: format!("{} was created successfully", display_name),
                            data: Some(parsed_data),
                        }
                    }
                    Err(err) => SuperSwarmResponse {
                        success: false,
                        message: err.to_string(),
                        data: None,
                    },
                };
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed (pure external I/O)
            SwarmCmd::StopEc2Instance(info) => {
                let res: SuperSwarmResponse;
                match crate::ec2::stop_ec2_instance_and_tag(&info.instance_id).await {
                    Ok(()) => {
                        res = SuperSwarmResponse {
                            success: true,
                            message: format!(
                                "Instance {} stopped successfully and tagged with DeletedOn",
                                info.instance_id
                            ),
                            data: None,
                        }
                    }
                    Err(err) => {
                        res = SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        }
                    }
                }
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 5: Read state, do I/O, write result back
            SwarmCmd::UpdateAwsInstanceType(info) => {
                let res = update_aws_instance_type(info, proj).await;
                let res: SuperSwarmResponse = match res {
                    Ok(_) => SuperSwarmResponse {
                        success: true,
                        message: "Instance updated successfully".to_string(),
                        data: None,
                    },
                    Err(err) => SuperSwarmResponse {
                        success: false,
                        message: err.to_string(),
                        data: None,
                    },
                };
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 1: Read and return
            SwarmCmd::GetInstanceType(info) => {
                let res: SuperSwarmResponse = state_read(|s| match get_swarm_instance_type(info.clone(), s) {
                    Ok(result) => result,
                    Err(err) => SuperSwarmResponse {
                        success: false,
                        message: err.to_string(),
                        data: None,
                    },
                })
                .await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::GetSwarmChildImageVersions(info) => {
                let swarm = state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved))
                    .await;
                let res = match swarm {
                    Some(swarm) => match get_child_swarm_image_versions(&swarm).await {
                        Ok(result) => result,
                        Err(err) => SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        },
                    },
                    None => SuperSwarmResponse {
                        success: false,
                        message: "Swarm does not exist".to_string(),
                        data: None,
                    },
                };
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::ChangeChildSwarmPassword(info) => {
                let swarm = state_read(|s| s.find_swarm_by_host(&info.host, info.is_reserved))
                    .await;
                let res: SuperSwarmResponse = update_swarm_child_password(info, swarm).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::GetLightningBotsDetails => {
                let bots = state_read(|s| s.lightning_bots.clone()).await;
                let res: SuperSwarmResponse = get_lightning_bots_details(bots).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 2: Mutate and return
            SwarmCmd::ChangeLightningBotLabel(info) => {
                let res: SuperSwarmResponse = state_write(proj, |s| {
                    change_lightning_bot_label(s, info)
                })
                .await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::CreateInvoiceForLightningBot(info) => {
                let bot = state_read(|s| {
                    s.lightning_bots
                        .iter()
                        .find(|b| b.url == info.id)
                        .cloned()
                })
                .await;
                let res: SuperSwarmResponse = create_invoice_lightning_bot(bot, info).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed
            SwarmCmd::GetSuperAdminLogs => {
                let res = get_super_admin_docker_logs().await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::UpdateChildSwarmEnv(data) => {
                let swarm =
                    state_read(|s| s.find_swarm_by_host(&data.host, data.is_reserved)).await;
                let res = update_child_swarm_env(swarm, data).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 2: Mutate and return
            SwarmCmd::AddAnthropicKey(data) => {
                let res = state_write(proj, |s| handle_add_anthropic_key(s, data)).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 1: Read and return
            SwarmCmd::GetAnthropicKey => {
                let res = state_read(|s| handle_get_anthropic_keys(s)).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed
            SwarmCmd::RestartSuperAdmin => {
                let res = update_super_admin().await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed
            SwarmCmd::GetSslCertExpiry => {
                let res = handle_get_ssl_cert_expiry().await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed
            SwarmCmd::RenewSslCert => {
                let res = match renew_cert().await {
                    Ok(data) => data,
                    Err(err) => SuperRestarterResponse {
                        ok: false,
                        message: Some(err.to_string()),
                        error: Some(err.to_string()),
                    },
                };
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed
            SwarmCmd::UploadSSlCert => {
                let res = match upload_cert_to_s3().await {
                    Ok(data) => data,
                    Err(err) => SuperRestarterResponse {
                        ok: false,
                        message: Some(err.to_string()),
                        error: Some(err.to_string()),
                    },
                };
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 5: Read state, do I/O, write result back
            SwarmCmd::UpdateChildSwarmPublicIp(info) => {
                let res = handle_update_child_swarm_public_ip(proj, info).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 1: Read and return
            SwarmCmd::GetChildSwarmCredentials(req) => {
                let res = state_read(|s| get_child_swarm_credentials(req, s)).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 3: Read state, do I/O, return
            SwarmCmd::NukeWarmSwarm(req) => {
                let state = state_read(|s| s.clone()).await;
                let res = nuke_warm_swarm_by_host(&req.host, &state).await;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::NukeAllWarmSwarms => {
                let state = state_read(|s| s.clone()).await;
                let res = nuke_all_warm_swarms(&state).await;
                Some(serde_json::to_string(&res)?)
            }
            // Pattern 4: No state needed (pure external I/O)
            SwarmCmd::GetEc2CpuUtilization(req) => {
                let res: SuperSwarmResponse =
                    match crate::cloudwatch::get_cpu_utilization(&req.instance_id).await {
                        Ok(Some(val)) => SuperSwarmResponse {
                            success: true,
                            message: "ok".to_string(),
                            data: Some(serde_json::json!({ "cpu_percent": val })),
                        },
                        Ok(None) => SuperSwarmResponse {
                            success: true,
                            message: "no data".to_string(),
                            data: None,
                        },
                        Err(err) => SuperSwarmResponse {
                            success: false,
                            message: err.to_string(),
                            data: None,
                        },
                    };
                Some(serde_json::to_string(&res)?)
            }
        },
    };

    Ok(ret.context("internal error")?)
}

pub fn fmt_err(err: &str) -> String {
    format!("{{\"stack_error\":\"{}\"}}", err.to_string())
}
