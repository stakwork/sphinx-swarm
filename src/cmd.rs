use crate::images::Image;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Cmd {
    Swarm(SwarmCmd),
    Relay(RelayCmd),
    Bitcoind(BitcoindCmd),
    Lnd(LndCmd),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SwarmCmd {
    GetConfig,
    AddNode(Image),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum RelayCmd {
    ListUsers,
    AddUser,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestMine {
    pub blocks: u64,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum BitcoindCmd {
    GetInfo,
    TestMine(TestMine),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum LndCmd {
    GetInfo,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::BtcImage;

    #[test]
    fn test_cmd() {
        let btc = BtcImage::new("bicoind", "regtest", "user");
        let c = Cmd::Swarm(SwarmCmd::AddNode(Image::Btc(btc)));
        println!("{}", serde_json::to_string(&c).unwrap());

        let c2 = Cmd::Relay(RelayCmd::AddUser);
        println!("{}", serde_json::to_string(&c2).unwrap());

        let c3 = Cmd::Swarm(SwarmCmd::GetConfig);
        println!("{}", serde_json::to_string(&c3).unwrap());

        assert!(true == true)
    }
}
