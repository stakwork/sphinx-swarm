use crate::images::Image;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Cmd {
    Swarm(SwarmCmd),
    Relay(RelayCmd),
    Bitcoind(BitcoindCmd),
    Lnd(LndCmd),
    Proxy(ProxyCmd),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SwarmCmd {
    GetConfig,
    AddNode(Image),
    GetContainerLogs(String),
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
    pub address: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddPeer {
    pub pubkey: String,
    pub host: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddChannel {
    pub pubkey: String,
    pub amount: i64,
    pub satsperbyte: u64,
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
    ListChannels,
    AddPeer(AddPeer),
    AddChannel(AddChannel),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProxyCmd {
    GetBalance,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::BtcImage;

    #[test]
    fn test_cmd() {
        let btc = BtcImage::new("bicoind", "23.0", "regtest", "user");
        let c = Cmd::Swarm(SwarmCmd::AddNode(Image::Btc(btc)));
        println!("{}", serde_json::to_string(&c).unwrap());

        let c2 = Cmd::Relay(RelayCmd::AddUser);
        println!("{}", serde_json::to_string(&c2).unwrap());

        let c3 = Cmd::Swarm(SwarmCmd::GetConfig);
        println!("{}", serde_json::to_string(&c3).unwrap());

        assert!(true == true)
    }
}
