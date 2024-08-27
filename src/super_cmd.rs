use serde::{Deserialize, Serialize};

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
    pub username: String,
    pub password: String,
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
    pub token: String,
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
}