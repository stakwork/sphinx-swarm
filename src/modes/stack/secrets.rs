use crate::utils;
use rand::{distributions::Alphanumeric, Rng, RngCore};
use std::collections::HashMap;

pub type Secrets = HashMap<String, String>;

fn random_secrets() -> Secrets {
    // store key hex
    let mut store_key_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut store_key_bytes);
    let store_key = hex::encode(store_key_bytes).to_uppercase();
    let mut s = HashMap::new();
    s.insert("bitcoin_pass".to_string(), random_word(12));
    s.insert("lnd1_password".to_string(), random_word(12));
    s.insert("proxy_admin_token".to_string(), random_word(12));
    s.insert("proxy_store_key".to_string(), store_key.to_string());
    s
}

fn random_word(n: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

pub fn load_secrets(project: &str) -> Secrets {
    let def = random_secrets();
    let path = format!("vol/{}/secrets.json", project);
    utils::load_json(&path, def)
}
fn get_secrets(project: &str) -> Secrets {
    let path = format!("vol/{}/secrets.json", project);
    utils::get_json(&path)
}
fn put_secrets(project: &str, rs: &Secrets) {
    let path = format!("vol/{}/secrets.json", project);
    utils::put_json(&path, rs)
}
pub fn add_to_secrets(project: &str, key: &str, val: &str) {
    let mut secrets = get_secrets(project);
    secrets.insert(key.to_string(), val.to_string());
    put_secrets(project, &secrets);
}
