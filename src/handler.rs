use std::collections::HashMap;

use crate::auth;
use crate::builder;
use crate::cmd::*;
use crate::config;
use crate::config::{Clients, Node, Stack, State, STATE};
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

// tag is the service name
pub async fn handle(proj: &str, cmd: Cmd, tag: &str, docker: &Docker) -> Result<String> {
    // conf can be mutated in place
    let mut state = config::STATE.lock().await;
    // println!("STACK {:?}", stack);

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
            SwarmCmd::AddNode(node) => {
                log::info!("AddNode -> {:?}", node);
                // add a node via docker
                None
            }
            SwarmCmd::UpdateNode(un) => {
                log::info!("UpdateNode -> {}", un.id);
                builder::update_node_and_make_client(proj, &docker, &un.id, &mut state).await?;
                // must_save_stack = true; // no "version" now. Its always "latest"
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
            SwarmCmd::AddBoltwallAdminPubkey(apk) => {
                log::info!("AddBoltwallAdminPubkey -> {}", apk);
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::add_admin_pubkey(&boltwall, &apk).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::GetBoltwallSuperAdmin => {
                log::info!("GetBoltwallSuperAdmin");
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::get_super_admin(&boltwall).await?;
                Some(serde_json::to_string(&response)?)
            }
            SwarmCmd::AddBoltwallSubAdminPubkey(apk) => {
                log::info!("AddBoltwallSubAdminPubkey -> {}", apk);
                let boltwall = find_boltwall(&state.stack.nodes)?;
                let response = crate::conn::boltwall::add_subadmin_pubkey(&boltwall, &apk).await?;
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
                let response = crate::conn::boltwall::delete_sub_admin(&boltwall, &apk).await?;
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
                    let result = client.add_peer(peer).await?;
                    Some(serde_json::to_string(&result)?)
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
                    let result = client.connect_peer(&peer.pubkey, &host, port).await?;
                    Some(serde_json::to_string(&result)?)
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
fn find_boltwall(nodes: &Vec<Node>) -> Result<BoltwallImage> {
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
                    handle(&project, cmd, &msg.tag, &docker),
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
