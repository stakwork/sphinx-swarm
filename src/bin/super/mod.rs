use anyhow::Result;
use serde::{Deserialize, Serialize};
use sphinx_swarm::config::User;
use sphinx_swarm::secrets;
use sphinx_swarm::utils;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Super {
    pub stacks: Vec<RemoteStack>,
    pub users: Vec<User>,
    pub jwt_key: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RemoteStack {
    pub host: String,
    pub user: String,
    pub pass: String,
}

#[rocket::main]
async fn main() -> Result<()> {
    let project = "super";
    let s: Super = load_config_file(project).await.expect("YAML CONFIG FAIL");
    println!("SUPER! {:?}", s);

    Ok(())
}

pub async fn load_config_file(project: &str) -> Result<Super> {
    let yaml_path = format!("vol/{}/config.yaml", project);
    let s = utils::load_yaml(&yaml_path, Default::default()).await?;
    Ok(s)
}

pub async fn put_config_file(project: &str, rs: &Super) {
    let path = format!("vol/{}/config.yaml", project);
    utils::put_yaml(&path, rs).await;
}

impl Default for Super {
    fn default() -> Self {
        Self {
            stacks: Vec::new(),
            users: vec![default_superuser()],
            jwt_key: secrets::random_word(16),
        }
    }
}

fn default_superuser() -> User {
    let username = "super";
    let default_password = "superpass123";
    let pass_hash = bcrypt::hash(default_password, bcrypt::DEFAULT_COST).expect("failed to bcrypt");
    User {
        id: 1,
        username: username.to_string(),
        pass_hash,
    }
}
