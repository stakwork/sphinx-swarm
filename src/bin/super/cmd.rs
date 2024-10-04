use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Cmd {
    Swarm(SwarmCmd),
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
pub struct AddNewSwarmInfo {
    pub host: String,
    pub instance: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddNewSwarmInfoAPI {
    pub host: String,
    pub instance: String,
    pub description: String,
    pub username: String,
    pub password: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateSwarmInfo {
    pub id: String,
    pub host: String,
    pub instance: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteSwarmInfo {
    pub host: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChildSwarm {
    pub password: String,
    pub host: String,
    pub username: String,
    pub token: String,
    pub default_host: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "content")]
pub enum SwarmCmd {
    GetConfig,
    Login(LoginInfo),
    ChangePassword(ChangePasswordInfo),
    AddNewSwarm(AddNewSwarmInfo),
    UpdateSwarm(UpdateSwarmInfo),
    DeleteSwarm(DeleteSwarmInfo),
    SetChildSwarm(ChildSwarm),
    GetChildSwarmConfig(ChildSwarmIdentifier),
    GetChildSwarmContainers(ChildSwarmIdentifier),
    StopChildSwarmContainers(AccessNodesInfo),
    StartChildSwarmContainers(AccessNodesInfo),
    UpdateChildSwarmContainers(AccessNodesInfo),
    RestartChildSwarmContainers(AccessNodesInfo),
    CreateNewEc2Instance(CreateEc2InstanceInfo),
    GetAwsInstanceTypes,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChildSwarmIdentifier {
    pub host: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddSwarmResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SuperSwarmResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessNodesInfo {
    pub host: String,
    pub nodes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateEc2InstanceInfo {
    pub name: String,
    pub vanity_address: Option<String>,
    pub instance_type: String,
}
