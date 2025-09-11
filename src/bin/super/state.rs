use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use sphinx_swarm::config::{Role, User};
use sphinx_swarm::secrets;
use sphinx_swarm::utils::getenv;

use crate::util::{get_descriptive_instance_type, get_today_dash_date};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Super {
    pub stacks: Vec<RemoteStack>,
    pub users: Vec<User>,
    pub jwt_key: String,
    pub bots: Vec<BotCred>,
    pub ec2_limit: Ec2Limit,
    pub lightning_bots: Vec<LightningBot>,
    pub reserved_domains: Option<Vec<String>>,
    pub reserved_instances: Option<ReservedInstances>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct RemoteStack {
    pub host: String,
    pub note: Option<String>,
    pub ec2: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub default_host: String,
    pub ec2_instance_id: String,
    pub public_ip_address: Option<String>,
    pub private_ip_address: Option<String>,
    pub id: Option<String>,
    pub deleted: Option<bool>,
    pub route53_domain_names: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default, Clone)]
pub struct Ec2Limit {
    pub count: i32,
    pub date: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default, Clone)]
pub struct ReservedInstances {
    pub minimum_available: i32,
    pub available_instances: Vec<AvailableInstances>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default, Clone)]
pub struct AvailableInstances {
    pub instance_id: String,
    pub instance_type: String,
    pub swarm_number: String,
    pub default_host: String,
    pub host: String,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub ip_address: Option<String>,
    pub admin_password: String,
    pub x_api_key: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default, Clone)]
pub struct InstanceFromAws {
    pub instance_id: String,
    pub instance_type: String,
    pub public_ip_address: String,
    pub private_ip_address: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct LightningBot {
    pub url: String,
    pub token: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct LightningBotsDetails {
    pub balance_in_msat: u64,
    pub contact_info: String,
    pub alias: String,
    pub network: String,
    pub error_message: String,
    pub id: String,
    pub label: String,
}

impl Default for Super {
    fn default() -> Self {
        Self {
            stacks: Vec::new(),
            users: vec![default_superuser()],
            jwt_key: secrets::random_word(16),
            bots: Vec::new(),
            ec2_limit: default_ec2_limit(),
            lightning_bots: Vec::new(),
            reserved_domains: Some(Vec::new()),
            reserved_instances: Some(default_reserved_instances()),
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

pub fn default_reserved_instances() -> ReservedInstances {
    let minimum_reserver = getenv("MINIMUM_RESERVED_INSTANCES")
        .unwrap_or("1".to_string())
        .parse::<i32>()
        .unwrap_or(1);
    ReservedInstances {
        minimum_available: minimum_reserver,
        available_instances: Vec::new(),
    }
}

fn default_ec2_limit() -> Ec2Limit {
    let today_dash_date = get_today_dash_date();
    Ec2Limit {
        count: 0,
        date: today_dash_date,
    }
}

impl Super {
    pub fn remove_tokens(&self) -> Super {
        let stacks = self
            .stacks
            .iter()
            .filter(|s| s.deleted == Some(false))
            .map(|n| RemoteStack {
                host: n.host.clone(),
                note: n.note.clone(),
                ec2: Some(get_descriptive_instance_type(n.ec2.clone())),
                user: None,
                pass: None,
                default_host: n.default_host.clone(),
                ec2_instance_id: n.ec2_instance_id.clone(),
                public_ip_address: n.public_ip_address.clone(),
                private_ip_address: n.private_ip_address.clone(),
                id: n.id.clone(),
                deleted: n.deleted.clone(),
                route53_domain_names: n.route53_domain_names.clone(),
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
            ec2_limit: Ec2Limit {
                count: 0,
                date: "".to_string(),
            },
            lightning_bots: vec![],
            reserved_domains: Some(vec![]),
            reserved_instances: self.reserved_instances.clone(),
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

    pub fn find_swarm_by_default_host(&self, default_host: &str) -> Option<&RemoteStack> {
        let pos = self
            .stacks
            .iter()
            .position(|s| s.default_host == default_host);
        if let None = pos {
            return None;
        }
        let pos = pos.unwrap();

        let swarm = &self.stacks[pos];

        Some(swarm)
    }

    pub fn find_swarm_by_id(&self, id: &str) -> Option<&RemoteStack> {
        let pos = self.stacks.iter().position(|s| s.id.as_deref() == Some(id));
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
