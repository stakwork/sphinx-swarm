use crate::utils;
use rand::{distributions::Alphanumeric, Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct Secrets {
    pub bitcoind_pass: String,
    pub lnd1_mnemonic: Option<Vec<String>>,
    pub lnd1_password: String,
    pub proxy_admin_token: String,
    pub proxy_store_key: String,
}

fn random_secrets() -> Secrets {
    // store key hex
    let mut store_key_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut store_key_bytes);
    let store_key = hex::encode(store_key_bytes).to_uppercase();
    Secrets {
        bitcoind_pass: random_word(12),
        lnd1_mnemonic: None,
        lnd1_password: random_word(12),
        proxy_admin_token: random_word(12),
        proxy_store_key: store_key.to_string(),
    }
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
pub fn add_mnemonic_to_secrets(project: &str, mnemonic: Vec<String>) {
    let mut secrets = get_secrets(project);
    secrets.lnd1_mnemonic = Some(mnemonic);
    put_secrets(project, &secrets);
}
