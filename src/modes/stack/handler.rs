use crate::cmd::{BitcoindCmd, Cmd, LndCmd, RelayCmd, SwarmCmd};
use crate::config::{Node, Stack, STATE};
use crate::conn::lnd::lndrpc::LndChannel;
use crate::images::Image;
use anyhow::{anyhow, Result};
use bollard::Docker;

// tag is the service name
pub async fn handle(cmd: Cmd, tag: &str, docker: &Docker) -> Result<String> {
    // conf can be mutated in place
    let mut state = STATE.lock().await;
    let mut stack = &state.stack;
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
        },
        Cmd::Relay(c) => match c {
            RelayCmd::AddUser => {
                // hit new relay add user in proxy route
                None
            }
            RelayCmd::ListUsers => None,
        },
        Cmd::Bitcoind(c) => {
            let client = state.clients.bitcoind.get(tag);
            if let None = client {
                return Err(anyhow!("no bitcoind client".to_string()));
            }
            // safe to unwrap here because "None" was already checked
            let client = client.unwrap();
            match c {
                BitcoindCmd::GetInfo => {
                    let info = client.get_info()?;
                    Some(serde_json::to_string(&info)?)
                }
            }
        }
        Cmd::Lnd(c) => {
            let client = state.clients.lnd.get_mut(tag);
            if let None = client {
                return Err(anyhow!("no lnd client".to_string()));
            }
            let client = client.unwrap();
            match c {
                LndCmd::GetInfo => {
                    let info = client.get_info().await?;
                    Some(serde_json::to_string(&info)?)
                },
                LndCmd::ListChannels => {
                    let mut lnd_channels:Vec<LndChannel> = Vec::new();

                    let channel_list = client.list_channels().await?;
                    let channels = channel_list.channels;

                    for chan in channels {
                        let new_chan = LndChannel::convert_to_json(chan);

                        lnd_channels.push(new_chan)
                    }

                    // println!("Channels {:?}", channelList);

                    Some(serde_json::to_string(&lnd_channels)?)
                }
            }
        }
    };
    match ret {
        Some(r) => Ok(r),
        None => Err(anyhow::anyhow!("internal error".to_string())),
    }
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
        },
    });
    Stack {
        network: s.network.clone(),
        nodes: nodes.collect(),
    }
}
