use std::collections::HashMap;

use crate::images::Image;
use serde::{Deserialize, Serialize};
use sphinx_auther::secp256k1::PublicKey;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Cmd {
    Swarm(SwarmCmd),
    Relay(RelayCmd),
    Bitcoind(BitcoindCmd),
    Lnd(LndCmd),
    Cln(ClnCmd),
    Proxy(ProxyCmd),
    Hsmd(HsmdCmd),
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
pub struct ChangeAdminInfo {
    pub user_id: u32,
    pub old_pass: String,
    pub password: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateNode {
    pub id: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdatePaidEndpointRequest {
    pub id: u64,
    pub status: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddUserRequest {
    pub role: u32,
    pub pubkey: String,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddAdminRequest {
    pub pubkey: String,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateAdminPubkeyInfo {
    pub user_id: u32,
    pub pubkey: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSecondBrainAboutRequest {
    pub app_version: String,
    pub description: String,
    pub mission_statement: String,
    pub search_term: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignUpAdminPubkeyDetails {
    pub challenge: String,
    pub user_id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeatureFlagUserRoles {
    pub user: bool,
    pub admin: bool,
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
    ChangeAdmin(ChangeAdminInfo),
    ListContainers,
    StartContainer(String),
    StopContainer(String),
    UpdateNode(UpdateNode),
    GetStatistics(Option<String>),
    AddBoltwallAdminPubkey(AddAdminRequest),
    GetBoltwallSuperAdmin,
    AddBoltwallUser(AddUserRequest),
    ListAdmins,
    DeleteSubAdmin(String),
    ListPaidEndpoint,
    UpdatePaidEndpoint(UpdatePaidEndpointRequest),
    UpdateSwarm,
    UpdateBoltwallAccessibility(bool),
    GetBoltwallAccessibility,
    UpdateAdminPubkey(UpdateAdminPubkeyInfo),
    GetFeatureFlags,
    GetSecondBrainAboutDetails,
    UpdateSecondBrainAbout(UpdateSecondBrainAboutRequest),
    UpdateFeatureFlags(HashMap<String, FeatureFlagUserRoles>),
    SignUpAdminPubkey(SignUpAdminPubkeyDetails),
    GetImageDigest(String),
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
#[serde(tag = "cmd", content = "content")]
pub enum RelayCmd {
    ListUsers,
    AddUser(AddUser),
    GetChats,
    AddDefaultTribe(DefaultTribe),
    RemoveDefaultTribe(DefaultTribe),
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
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PayKeysend {
    pub amt: i64,
    pub dest: String,
    pub route_hint: Option<String>,
    pub maxfeepercent: Option<f64>,
    pub exemptfee: Option<u64>,
    pub tlvs: Option<HashMap<u64, Vec<u8>>>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CloseChannel {
    pub id: String,
    pub destination: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddChannel {
    pub pubkey: String,
    pub amount: i64,
    pub satsperbyte: u64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetInvoice {
    pub payment_hash: Option<String>,
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
    ListPayments,
    ListInvoices,
    ListPendingChannels,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum ClnCmd {
    GetInfo,
    ListPeers,
    ListFunds,
    NewAddress,
    AddPeer(AddPeer),
    AddChannel(AddChannel),
    AddInvoice(AddInvoice),
    PayInvoice(PayInvoice),
    PayKeysend(PayKeysend),
    CloseChannel(CloseChannel),
    ListInvoices(Option<GetInvoice>),
    ListPays(Option<GetInvoice>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum ProxyCmd {
    GetBalance,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum HsmdCmd {
    GetClients,
}

impl Cmd {
    pub fn can_run_before_ready(&self) -> bool {
        match self {
            Cmd::Swarm(c) => match c {
                SwarmCmd::GetConfig => true,
                SwarmCmd::Login(_) => true,
                _ => false,
            },
            _ => false,
        }
    }
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
