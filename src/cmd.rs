use crate::config::Node;
use crate::images::{BtcImage, Image};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Cmd {
    Swarm(SwarmCmd),
    Relay(RelayCmd),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SwarmCmd {
    GetConfig,
    AddNode(Node),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum RelayCmd {
    AddUser,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd() {
        let btc = BtcImage::new("bicoind", "regtest", "user", "password");
        let c = Cmd::Swarm(SwarmCmd::AddNode(Node::new(Image::Btc(btc), vec![])));
        println!("{}", serde_json::to_string(&c).unwrap());

        let c2 = Cmd::Relay(RelayCmd::AddUser);
        println!("{}", serde_json::to_string(&c2).unwrap());

        let c3 = Cmd::Swarm(SwarmCmd::GetConfig);
        println!("{}", serde_json::to_string(&c3).unwrap());

        assert!(true == true)
    }
}
