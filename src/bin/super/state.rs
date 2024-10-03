use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use sphinx_swarm::config::{Role, User};
use sphinx_swarm::secrets;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Super {
    pub stacks: Vec<RemoteStack>,
    pub users: Vec<User>,
    pub jwt_key: String,
    pub bots: Vec<BotCred>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct RemoteStack {
    pub host: String,
    pub note: Option<String>,
    pub ec2: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub default_host: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct AwsInstanceType {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct BotCred {
    pub bot_id: String,
    pub bot_secret: String,
    pub chat_pubkey: String,
    pub bot_url: String,
}

impl Default for Super {
    fn default() -> Self {
        Self {
            stacks: Vec::new(),
            users: vec![default_superuser()],
            jwt_key: secrets::random_word(16),
            bots: Vec::new(),
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
    let default_password = "superpass";
    let pass_hash = bcrypt::hash(default_password, bcrypt::DEFAULT_COST).expect("failed to bcrypt");
    User {
        id: 1,
        username: username.to_string(),
        pass_hash,
        pubkey: None,
        role: Role::Super,
    }
}

impl Super {
    pub fn remove_tokens(&self) -> Super {
        let stacks = self
            .stacks
            .iter()
            .map(|n| RemoteStack {
                host: n.host.clone(),
                note: n.note.clone(),
                ec2: n.ec2.clone(),
                user: None,
                pass: None,
                default_host: n.default_host.clone(),
            })
            .collect();
        let bots = self
            .bots
            .iter()
            .map(|n| BotCred {
                bot_id: n.bot_id.clone(),
                bot_secret: "".to_string(),
                chat_pubkey: n.chat_pubkey.clone(),
                bot_url: n.bot_url.clone(),
            })
            .collect();
        Super {
            stacks: stacks,
            users: vec![],
            jwt_key: "".to_string(),
            bots: bots,
        }
    }

    pub fn add_remote_stack(&mut self, new_stack: RemoteStack) {
        self.stacks.push(new_stack);
    }

    pub fn find_swarm_by_host(&self, host: &str) -> Option<&RemoteStack> {
        let pos = self.stacks.iter().position(|s| s.host == host);
        if let None = pos {
            return None;
        }
        let pos = pos.unwrap();

        let swarm = &self.stacks[pos];

        Some(swarm)
    }

    pub fn delete_swarm_by_host(&mut self, host: &str) -> Result<(), String> {
        let initial_len = self.stacks.len();
        self.stacks.retain(|stack| stack.host != host);

        if self.stacks.len() == initial_len {
            Err(format!("Host '{}' does not exist.", host))
        } else {
            Ok(())
        }
    }
}
