use std::collections::HashMap;

use anyhow::{Context, Result};
use bollard::Docker;
use serde::{Deserialize, Serialize};
use sphinx_swarm::auth;
use sphinx_swarm::cmd::*;
use sphinx_swarm::config::{put_config_file, Node, Stack, STATE};
use sphinx_swarm::dock::container_logs;
use sphinx_swarm::dock::create_and_start;
use sphinx_swarm::dock::list_containers;
use sphinx_swarm::dock::start_container;
use sphinx_swarm::dock::stop_and_remove;
use sphinx_swarm::dock::stop_container;
use sphinx_swarm::images;
use sphinx_swarm::images::boltwall::BoltwallImage;
use sphinx_swarm::images::btc::BtcImage;
use sphinx_swarm::images::cache::CacheImage;
use sphinx_swarm::images::jarvis::JarvisImage;
use sphinx_swarm::images::lnd::LndImage;
use sphinx_swarm::images::navfiber::NavFiberImage;
use sphinx_swarm::images::neo4j::Neo4jImage;
use sphinx_swarm::images::proxy::ProxyImage;
use sphinx_swarm::images::relay::RelayImage;
use sphinx_swarm::images::{DockerHubImage, Image};
use sphinx_swarm::secrets;

// tag is the service name
pub async fn handle(proj: &str, cmd: Cmd, tag: &str, docker: &Docker) -> Result<String> {
    // conf can be mutated in place
    let mut state = STATE.lock().await;
    // println!("STACK {:?}", stack);

    let mut must_save_stack = false;

    let ret: Option<String> = match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::GetConfig => {
                let res = remove_tokens(&state.stack);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::AddNode(_node) => {
                // add a node via docker
                None
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
            SwarmCmd::ListContainers => {
                let containers = list_containers(docker).await?;
                Some(serde_json::to_string(&containers)?)
            }
            SwarmCmd::StartContainer(id) => {
                let res = start_container(docker, &id).await?;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::StopContainer(id) => {
                let res = stop_container(docker, &id).await?;
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::UpdateNode(node) => {
                /* Check if the npde is a running node
                 * if it does not return error
                 */
                let action_node = state
                    .stack
                    .nodes
                    .iter()
                    .find(|n| n.name() == node.id.clone())
                    .context("Node not found")?
                    .as_internal()?;

                stop_and_remove(docker, &node.id.clone()).await?;

                let mut new_node;

                match action_node.typ().as_str() {
                    "Btc" => {
                        let old_btc = action_node.as_btc()?;
                        let mut btc = BtcImage::new(
                            &old_btc.name,
                            &node.version,
                            &old_btc.network,
                            &old_btc.user,
                        );
                        btc.set_password(&old_btc.pass);
                        new_node = images::btc::btc(&btc);
                    }
                    "Lnd" => {
                        let old_lnd = action_node.as_lnd()?;
                        let mut lnd = LndImage::new(
                            &old_lnd.name,
                            &node.version,
                            &old_lnd.network,
                            &old_lnd.rpc_port,
                            &old_lnd.peer_port,
                        );
                        if let Some(http_port) = old_lnd.http_port {
                            lnd.http_port = Some(http_port);
                        }
                        let links: Vec<&str> = toVecStr(&old_lnd.links);
                        lnd.links(links);
                        lnd.host(old_lnd.host);
                    }
                    "Relay" => {
                        let old_relay = action_node.as_relay()?;

                        let mut relay = RelayImage::new(
                            &old_relay.name,
                            &node.version,
                            &old_relay.node_env,
                            &old_relay.port,
                        );
                        let links: Vec<&str> = toVecStr(&old_relay.links);
                        relay.links(links);
                        relay.host(old_relay.host.clone());
                        new_node = images::relay::relay(&relay);
                    }
                    "Proxy" => {
                        let old_proxy = action_node.as_proxy()?;
                        let mut proxy = ProxyImage::new(
                            &old_proxy.name,
                            &node.version,
                            &old_proxy.network,
                            &old_proxy.port,
                            &old_proxy.admin_port,
                        );

                        let links: Vec<&str> = toVecStr(&old_proxy.links);
                        proxy.new_nodes(Some("0".to_string()));
                        proxy.links(links);
                        new_node = images::proxy::proxy(proxy, lnd);
                    }
                    "Cache" => {
                        let old_cache = action_node.as_cache()?;
                        let mut cache =
                            CacheImage::new(&old_cache.name, &node.version, &old_cache.port, true);
                        let links: Vec<&str> = toVecStr(&old_cache.links);
                        cache.links(links);
                        new_node = images::cache::cache(cache, meme_host, mqtt_host)
                    }
                    "Neo4j" => {
                        let old_neo4j = action_node.as_neo4j()?;
                        let neo4j = Neo4jImage::new(&old_neo4j.name, &node.version);
                        new_node = images::neo4j::neo4j(&neo4j);
                    }
                    "NavFiber" => {
                        let old_nav = action_node.as_navfiber()?;
                        let mut nav =
                            NavFiberImage::new(&old_nav.name, &node.version, &old_nav.port);
                        let links: Vec<&str> = toVecStr(&old_nav.links);
                        nav.links(links);
                        nav.host(old_nav.host.clone());
                        new_node = images::navfiber::navfiber(nav);
                    }
                    "JarvisBackend" => {
                        let old_jarvis = action_node.as_jarvis()?;
                        let mut jarvis =
                            JarvisImage::new(&old_jarvis.name, &node.version, &old_jarvis.port);
                        let links: Vec<&str> = toVecStr(&old_jarvis.links);
                        jarvis.links(links);
                        new_node = images::jarvis::jarvis(jarvis, neo4j);
                    }
                    "Boltwall" => {
                        let old_bolt = action_node.as_boltwall()?;
                        let mut bolt =
                            BoltwallImage::new(&old_bolt.name, &node.version, &old_bolt.port);
                        let links: Vec<&str> = toVecStr(&old_bolt.links);
                        bolt.links(links);
                        bolt.host(old_bolt.host.clone());
                        new_node =
                            images::boltwall::boltwall(&bolt, macaroon, cert, lnd_node, jarvis);
                    }
                    _ => println!("Not a swarm node"),
                }

                // Start the node
                create_and_start(docker, new_node, false).await?;
                must_save_stack = true;

                let msg = format!("Updated {} node successfully", action_node.name());

                Some(serde_json::to_string(&msg)?)
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
                RelayCmd::CreateTribe(t) => Some(client.create_tribe(&t.name).await?.to_string()?),
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
                    Some(serde_json::to_string(&bal.confirmed_balance)?)
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
    };

    if must_save_stack {
        put_config_file(proj, &state.stack).await;
    }
    Ok(ret.context("internal error")?)
}

// remove sensitive data from Stack when sending over wire
fn remove_tokens(s: &Stack) -> Stack {
    let nodes = s.nodes.iter().map(|n| match n {
        Node::External(e) => Node::External(e.clone()),
        Node::Internal(i) => match i.clone() {
            Image::Btc(mut b) => {
                b.pass = "".to_string();
                Node::Internal(Image::Btc(b))
            }
            Image::Lnd(mut l) => {
                l.unlock_password = "".to_string();
                Node::Internal(Image::Lnd(l))
            }
            Image::Proxy(mut p) => {
                p.store_key = None;
                p.admin_token = None;
                Node::Internal(Image::Proxy(p))
            }
            Image::Relay(r) => Node::Internal(Image::Relay(r)),
            Image::Cache(c) => Node::Internal(Image::Cache(c)),
            Image::Neo4j(n) => Node::Internal(Image::Neo4j(n)),
            Image::NavFiber(nf) => Node::Internal(Image::NavFiber(nf)),
            Image::Jarvis(j) => Node::Internal(Image::Jarvis(j)),
            Image::BoltWall(mut b) => {
                b.session_secret = "".to_string();
                Node::Internal(Image::BoltWall(b))
            }
        },
    });
    Stack {
        network: s.network.clone(),
        nodes: nodes.collect(),
        host: s.host.clone(),
        users: vec![],
        jwt_key: "".to_string(),
    }
}

fn toVecStr(links: &Vec<String>) -> Vec<&str> {
    links.iter().map(|s| s as &str).collect()
}
