use crate::cmd::{Cmd, RelayCmd, SwarmCmd};
use crate::config::{Node, Stack, STACK};
use crate::images::Image;
use anyhow::Result;
use bollard::Docker;

// tag is the service name
pub async fn handle(cmd: Cmd, tag: &str, docker: &Docker) -> Result<String> {
    // conf can be mutated in place
    let mut stack = STACK.lock().await;
    // println!("STACK {:?}", stack);

    let ret: Option<String> = match cmd {
        Cmd::Swarm(c) => match c {
            SwarmCmd::GetConfig => {
                let res = remove_tokens(&*stack);
                Some(serde_json::to_string(&res)?)
            }
            SwarmCmd::AddNode(node) => {
                // add a node via docker
                None
            }
            SwarmCmd::GetBitcoinInfo => {
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
    };
    match ret {
        Some(r) => Ok(r),
        None => Err(anyhow::anyhow!("no return value".to_string())),
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
