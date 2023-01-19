use anyhow::{Context, Result};
use bollard::Docker;
use serde::{Deserialize, Serialize};
use sphinx_swarm::cmd::*;
use sphinx_swarm::config::{Node, Stack, STATE};
use sphinx_swarm::dock::container_logs;
use sphinx_swarm::images::Image;

// tag is the service name
pub async fn handle(cmd: Cmd, tag: &str, docker: &Docker) -> Result<String> {
    // conf can be mutated in place
    let mut state = STATE.lock().await;
    let stack = &state.stack;
    // println!("STACK {:?}", stack);

    let ret: Option<String> = match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::GetConfig => {
                let res = remove_tokens(stack);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::AddNode(node) => {
                // add a node via docker
                None
            }
            SwarmCmd::GetContainerLogs(container_name) => {
                let logs = container_logs(docker, &container_name).await;
                Some(serde_json::to_string(&logs)?)
            }
            SwarmCmd::ListVersions(req) => {
                // declare structs for json data format
                #[derive(Serialize, Deserialize, Debug, Clone)]
                struct ImageData {
                    org: String,
                    repo: String,
                    images: String,
                }

                // get nodes
                let nodes = stack.nodes.clone();
                let msg = format!("Couldn't find ({}) node", req.name);

                // find the node by name
                // get the image by calling node.repo()
                let node = nodes
                    .iter()
                    .find(|n| n.name() == req.name)
                    .context(msg)?
                    .as_internal()?
                    .repo();

                let org = node.org;
                let repo = node.repo;

                // return the json from
                let url = format!(
                    "https://hub.docker.com/v2/namespaces/{}/repositories/{}/tags?page={}",
                    org, repo, req.page
                );

                let body = reqwest::get(url).await?.text().await?;

                let data = ImageData {
                    org,
                    repo,
                    images: body,
                };

                Some(serde_json::to_string(&data)?)
            }
        },
        Cmd::Relay(c) => match c {
            RelayCmd::AddUser => {
                // hit new relay add user in proxy route
                None
            }
            RelayCmd::ListUsers => None,
        },
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
            Image::Cache(_) => todo!(),
        },
    });
    Stack {
        network: s.network.clone(),
        nodes: nodes.collect(),
    }
}
