use crate::images::Image;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Cmd {
    Swarm(SwarmCmd),
    Relay(RelayCmd),
    Bitcoind(BitcoindCmd),
    Lnd(LndCmd),
    Cln(ClnCmd),
    Proxy(ProxyCmd),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageRequest {
    pub name: String,
    pub page: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginInfo {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChangePasswordInfo {
    pub user_id: u32,
    pub old_pass: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateNode {
    pub id: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SwarmCmd {
    GetConfig,
    AddNode(Image),
    GetContainerLogs(String),
    ListVersions(ImageRequest),
    Login(LoginInfo),
    ChangePassword(ChangePasswordInfo),
    ListContainers,
    StartContainer(String),
    StopContainer(String),
    UpdateNode(UpdateNode),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddUser {
    pub initial_sats: Option<u64>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DefaultTribe {
    pub id: u16,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTribe {
    pub name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum RelayCmd {
    ListUsers,
    AddUser(AddUser),
    GetChats,
    AddDefaultTribe(DefaultTribe),
    RemoveDefaultTribe(DefaultTribe),
    CreateTribe(CreateTribe),
    GetToken,
    GetBalance,
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
pub struct AddInvoice {
    pub amt_paid_sat: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PayInvoice {
    pub payment_request: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PayKeysend {
    pub amt: i64,
    pub dest: String,
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
    GetBalance,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum LndCmd {
    GetInfo,
    ListChannels,
    ListPeers,
    AddPeer(AddPeer),
    AddChannel(AddChannel),
    NewAddress,
    GetBalance,
    AddInvoice(AddInvoice),
    PayInvoice(PayInvoice),
    PayKeysend(PayKeysend),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum ClnCmd {
    GetInfo,
    ListPeers,
    ListFunds,
    NewAddress,
    AddInvoice(AddInvoice),
    PayInvoice(PayInvoice),
    PayKeysend(PayKeysend),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum ProxyCmd {
    GetBalance,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::btc::BtcImage;

    #[test]
    fn test_cmd() {
        let btc = BtcImage::new("bicoind", "23.0", "regtest");
        let c = Cmd::Swarm(SwarmCmd::AddNode(Image::Btc(btc)));
        println!("{}", serde_json::to_string(&c).unwrap());

        // let c2 = Cmd::Relay(RelayCmd::AddUser);
        // println!("{}", serde_json::to_string(&c2).unwrap());

        let c3 = Cmd::Swarm(SwarmCmd::GetConfig);
        println!("{}", serde_json::to_string(&c3).unwrap());

        assert!(true == true)
    }
}
