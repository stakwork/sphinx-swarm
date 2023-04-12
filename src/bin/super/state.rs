use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use sphinx_swarm::config::User;
use sphinx_swarm::secrets;

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

impl Default for Super {
    fn default() -> Self {
        Self {
            stacks: Vec::new(),
            users: vec![default_superuser()],
            jwt_key: secrets::random_word(16),
        }
    }
}

pub static STATE: Lazy<Mutex<Super>> = Lazy::new(|| Mutex::new(Default::default()));

pub async fn hydrate(sup: Super) {
    // set into the main state mutex
    let mut state = STATE.lock().await;
    *state = sup;
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

impl Super {
    pub fn remove_tokens(&self) -> Super {
        let stacks = self
            .stacks
            .iter()
            .map(|n| RemoteStack {
                host: n.host.clone(),
                user: "".to_string(),
                pass: "".to_string(),
            })
            .collect();
        Super {
            stacks: stacks,
            users: vec![],
            jwt_key: "".to_string(),
        }
    }
}
