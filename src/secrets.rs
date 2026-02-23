use crate::utils;
use rand::{distributions::Alphanumeric, Rng, RngCore};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub type Secrets = HashMap<String, String>;

pub fn hex_secret() -> String {
    let mut store_key_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut store_key_bytes);
    hex::encode(store_key_bytes).to_uppercase()
}

pub fn hex_secret_32() -> String {
    let mut store_key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut store_key_bytes);
    hex::encode(store_key_bytes)
}

pub fn random_word(n: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

/// SHA-256 hash, hex-encoded, truncated to 24 characters.
/// Used to derive a public-safe token from a secret.
pub fn sha256_hex_24(input: &str) -> String {
    let hash = Sha256::digest(input.as_bytes());
    hex::encode(hash)[..24].to_string()
}

pub async fn load_secrets(project: &str) -> Secrets {
    let path = format!("vol/{}/secrets.json", project);
    utils::load_json(&path, HashMap::new()).await
}
async fn get_secrets(project: &str) -> Secrets {
    let path = format!("vol/{}/secrets.json", project);
    utils::get_json(&path).await
}
async fn put_secrets(project: &str, rs: &Secrets) {
    let path = format!("vol/{}/secrets.json", project);
    utils::put_json(&path, rs).await
}
pub async fn add_to_secrets(project: &str, key: &str, val: &str) {
    let mut secrets = get_secrets(project).await;
    secrets.insert(key.to_string(), val.to_string());
    put_secrets(project, &secrets).await;
}
