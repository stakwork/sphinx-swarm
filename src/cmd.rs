use anyhow::Error;
use reqwest::Response;
use std::collections::HashMap;

use crate::{images::Image, utils::make_reqwest_client};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetDockerImageTagsDetails {
    pub page: String,
    pub page_size: String,
    pub org_image_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateUserDetails {
    pub name: String,
    pub pubkey: String,
    pub role: u32,
    pub id: u32,
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
    GetDockerImageTags(GetDockerImageTagsDetails),
    UpdateUser(UpdateUserDetails),
    GetApiToken,
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
pub struct SendCmdData {
    pub cmd: String,
    pub content: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CmdRequest {
    #[serde(rename = "type")]
    cmd_type: String,
    data: SendCmdData,
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

pub async fn send_cmd_request(
    cmd: Cmd,
    tag: &str,
    url: &str,
    header_name: Option<&str>,
    header_value: Option<&str>,
) -> Result<Response, Error> {
    // let request = CmdRequest { cmd_type, data };
    let txt = serde_json::to_string(&cmd).context("could not stringify request")?;

    let client = make_reqwest_client();

    let route = format!("{}/cmd", url);

    if let (Some(name), Some(value)) = (header_name, header_value) {
        return Ok(client
            .get(&route)
            .header(name, value)
            .query(&[("txt", txt.as_str()), ("tag", tag)])
            .send()
            .await?);
    }

    let res = client
        .get(route)
        .query(&[("txt", txt.as_str()), ("tag", tag)])
        .send()
        .await?;

    Ok(res)
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
