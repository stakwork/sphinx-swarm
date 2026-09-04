use anyhow::Error;
use reqwest::Response;
use std::collections::HashMap;

use crate::{config::LightningPeer, images::Image, utils::make_reqwest_client};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sphinx_auther::secp256k1::PublicKey;

/// Placeholder printed instead of secret values in `Debug` output, so that
/// `log::info!("=> CMD: {:?}", cmd)` never writes passwords or env values
/// to the log stream.
const REDACTED: &str = "[REDACTED]";

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

#[derive(Serialize, Deserialize, Clone)]
pub struct LoginInfo {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for LoginInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginInfo")
            .field("username", &self.username)
            .field("password", &REDACTED)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChangePasswordInfo {
    pub user_id: u32,
    pub old_pass: String,
    pub password: String,
}

impl std::fmt::Debug for ChangePasswordInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangePasswordInfo")
            .field("user_id", &self.user_id)
            .field("old_pass", &REDACTED)
            .field("password", &REDACTED)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChangeAdminInfo {
    pub user_id: u32,
    pub old_pass: String,
    pub password: String,
    pub email: String,
}

impl std::fmt::Debug for ChangeAdminInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeAdminInfo")
            .field("user_id", &self.user_id)
            .field("old_pass", &REDACTED)
            .field("password", &REDACTED)
            .field("email", &self.email)
            .finish()
    }
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
pub struct UpdateEndpointPriceRequest {
    pub id: u64,
    pub price: u64,
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
    pub host: Option<String>,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct ChangeUserPasswordBySuperAdminInfo {
    pub new_password: String,
    pub current_password: String,
    pub username: String,
}

impl std::fmt::Debug for ChangeUserPasswordBySuperAdminInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeUserPasswordBySuperAdminInfo")
            .field("new_password", &REDACTED)
            .field("current_password", &REDACTED)
            .field("username", &self.username)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoltwallUser {
    pub id: i64,
    pub pubkey: String,
    pub name: String,
    pub role: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetBoltwallUsersResponse {
    pub users: Vec<BoltwallUser>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestPerSecondsInfo {
    pub request_per_seconds: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MaxRequestLimitInfo {
    pub max_request_limit: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateNeo4jConfigRequest {
    pub heap_initial_gb: Option<u64>,
    pub heap_max_gb: Option<u64>,
    pub pagecache_gb: Option<u64>,
    pub tx_total_gb: Option<u64>,
    pub tx_max_gb: Option<u64>,
    pub checkpoint_iops: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateEnvRequest {
    pub id: Option<String>,
    pub values: HashMap<String, String>,
}

impl std::fmt::Debug for UpdateEnvRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // env values can hold passwords / API keys — log the keys only
        let keys: Vec<&String> = self.values.keys().collect();
        f.debug_struct("UpdateEnvRequest")
            .field("id", &self.id)
            .field("values", &keys)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AssignSwarmNewDetails {
    pub new_password: Option<String>,
    pub old_password: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

impl std::fmt::Debug for AssignSwarmNewDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let env_keys: Option<Vec<&String>> = self.env.as_ref().map(|m| m.keys().collect());
        f.debug_struct("AssignSwarmNewDetails")
            .field("new_password", &self.new_password.as_ref().map(|_| REDACTED))
            .field("old_password", &self.old_password.as_ref().map(|_| REDACTED))
            .field("env", &env_keys)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContainerLogsRequest {
    pub name: String,
    pub before_timestamp: Option<String>,
    pub since_timestamp: Option<String>,
}

pub const BOLTWALL_TABLES: &[&str] = &[
    "sphinx_lsat",
    "dynamic_lsat",
    "transaction",
    "top_up",
    "paid_endpoint",
    "sphinx_users",
    "sphinx_feature_flag",
    "about",
];

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SwarmCmd {
    GetConfig,
    AddNode(Image),
    GetContainerLogs(ContainerLogsRequest),
    ListVersions(ImageRequest),
    Login(LoginInfo),
    ChangePassword(ChangePasswordInfo),
    ChangeAdmin(ChangeAdminInfo),
    ListContainers,
    StartContainer(String),
    StopContainer(String),
    RestartContainer(String),
    UpdateNode(UpdateNode),
    GetStatistics(Option<String>),
    GetHostStorage,
    AddBoltwallAdminPubkey(AddAdminRequest),
    GetBoltwallSuperAdmin,
    AddBoltwallUser(AddUserRequest),
    ListAdmins,
    DeleteSubAdmin(String),
    ListPaidEndpoint,
    UpdatePaidEndpoint(UpdatePaidEndpointRequest),
    UpdateEndpointPrice(UpdateEndpointPriceRequest),
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
    SetGlobalMemLimit(u64),
    GetSignedInUserDetails,
    GetAllImageActualVersion,
    ChangeUserPasswordBySuperAdmin(ChangeUserPasswordBySuperAdminInfo),
    GetLightningPeers,
    AddLightningPeer(LightningPeer),
    UpdateLightningPeer(LightningPeer),
    GetNeo4jPassword,
    GetBotToken,
    GetBotBalance,
    GetBotPayments,
    CreateBotInvoice(CreateBotInvoiceRequest),
    GetL402Stats,
    GetAdminTransactions(AdminTransactionsRequest),
    GetBoltwallDbTable(String),
    GetBoltwallUsers,
    UpdateBoltwallRequestPerSeconds(RequestPerSecondsInfo),
    GetBoltwallRequestPerSeconds,
    GetBoltwallMaxRequestLimit,
    UpdateBoltwallMaxRequestLimit(MaxRequestLimitInfo),
    GetEnv(String),
    UpdateEvn(UpdateEnvRequest),
    ChangeReservedSwarmToActive(AssignSwarmNewDetails),
    UpdateSslCert,
    UpdateNeo4jConfig(UpdateNeo4jConfigRequest),
    HermesAuthStart(HermesAuthRequest),
    HermesAuthStatus(String),
    HermesAuthList(HermesAuthRequest),
    HermesAuthLogout(HermesAuthRequest),
}

/// `provider` defaults to "xai-oauth" when omitted.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HermesAuthRequest {
    pub provider: Option<String>,
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
    pub alias: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddInvoice {
    pub amt_paid_sat: i64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateBotInvoiceRequest {
    pub amt_msat: u64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdminTransactionsRequest {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
    pub endpoint: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L402StatsResponse {
    pub total_l402s: u64,
    pub total_remaining_balance: u64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BotBalanceRes {
    pub msat: u64,
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
    ListPeerChannels,
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

#[cfg(test)]
mod ready_tests {
    use super::*;

    #[test]
    fn get_host_storage_can_run_before_ready() {
        let cmd = Cmd::Swarm(SwarmCmd::GetHostStorage);
        assert!(cmd.can_run_before_ready());
        // existing allowlist unaffected
        assert!(Cmd::Swarm(SwarmCmd::GetConfig).can_run_before_ready());
        assert!(Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "u".into(),
            password: "p".into(),
        }))
        .can_run_before_ready());
        assert!(!Cmd::Swarm(SwarmCmd::ListContainers).can_run_before_ready());
    }
}

impl Cmd {
    pub fn can_run_before_ready(&self) -> bool {
        match self {
            Cmd::Swarm(c) => match c {
                SwarmCmd::GetConfig => true,
                SwarmCmd::Login(_) => true,
                SwarmCmd::GetHostStorage => true,
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

    #[test]
    fn login_debug_redacts_password() {
        let info = LoginInfo {
            username: "admin".to_string(),
            password: "s3cret-pass".to_string(),
        };
        let dbg = format!("{:?}", info);
        assert!(!dbg.contains("s3cret-pass"));
        assert!(dbg.contains("admin"));
        assert!(dbg.contains("[REDACTED]"));
    }

    #[test]
    fn change_password_debug_redacts_both_passwords() {
        let info = ChangePasswordInfo {
            user_id: 1,
            old_pass: "old-pass-123".to_string(),
            password: "new-pass-456".to_string(),
        };
        let dbg = format!("{:?}", info);
        assert!(!dbg.contains("old-pass-123"));
        assert!(!dbg.contains("new-pass-456"));
        assert!(dbg.contains("user_id: 1"));
        assert!(dbg.contains("[REDACTED]"));
    }

    #[test]
    fn change_admin_debug_redacts_passwords() {
        let info = ChangeAdminInfo {
            user_id: 2,
            old_pass: "old-pass-123".to_string(),
            password: "new-pass-456".to_string(),
            email: "a@b.c".to_string(),
        };
        let dbg = format!("{:?}", info);
        assert!(!dbg.contains("old-pass-123"));
        assert!(!dbg.contains("new-pass-456"));
        assert!(dbg.contains("a@b.c"));
    }

    #[test]
    fn change_user_password_debug_redacts_passwords() {
        let info = ChangeUserPasswordBySuperAdminInfo {
            new_password: "fresh-pass-789".to_string(),
            current_password: "current-pass-000".to_string(),
            username: "admin".to_string(),
        };
        let dbg = format!("{:?}", info);
        assert!(!dbg.contains("fresh-pass-789"));
        assert!(!dbg.contains("current-pass-000"));
        assert!(dbg.contains("admin"));
    }

    #[test]
    fn update_env_debug_logs_keys_only() {
        let mut values = HashMap::new();
        values.insert("HOST".to_string(), "https://hidden-host.example".to_string());
        values.insert(
            "NEO4J_PASSWORD".to_string(),
            "env-secret-value-1".to_string(),
        );
        let req = UpdateEnvRequest {
            id: Some("boltwall".to_string()),
            values,
        };
        let dbg = format!("{:?}", req);
        // keys are visible...
        assert!(dbg.contains("HOST"));
        assert!(dbg.contains("NEO4J_PASSWORD"));
        // ...but values never are
        assert!(!dbg.contains("env-secret-value-1"));
        assert!(!dbg.contains("hidden-host.example"));
    }

    #[test]
    fn assign_swarm_new_details_debug_redacts_passwords_and_env() {
        let mut env = HashMap::new();
        env.insert("OWNER_PUBKEY".to_string(), "assign-secret-value-2".to_string());
        let details = AssignSwarmNewDetails {
            new_password: Some("new-swarm-pass-1".to_string()),
            old_password: Some("old-swarm-pass-1".to_string()),
            env: Some(env),
        };
        let dbg = format!("{:?}", details);
        assert!(!dbg.contains("new-swarm-pass-1"));
        assert!(!dbg.contains("old-swarm-pass-1"));
        assert!(!dbg.contains("assign-secret-value-2"));
        assert!(dbg.contains("OWNER_PUBKEY"));
    }

    #[test]
    fn swarm_cmd_debug_redacts_nested_secrets() {
        // the "=> CMD: {:?}" log path formats the whole command enum
        let cmd = Cmd::Swarm(SwarmCmd::Login(LoginInfo {
            username: "admin".to_string(),
            password: "s3cret-pass".to_string(),
        }));
        let dbg = format!("{:?}", cmd);
        assert!(!dbg.contains("s3cret-pass"));
        assert!(dbg.contains("admin"));
        assert!(dbg.contains("Login"));
    }

    #[test]
    fn redacted_structs_still_serialize_real_values() {
        // Debug is redacted for logs; serde must still carry the real payload
        let info = LoginInfo {
            username: "admin".to_string(),
            password: "s3cret-pass".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("s3cret-pass"));
        let back: LoginInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.password, "s3cret-pass");
    }
}
