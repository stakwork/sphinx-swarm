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
    let path_str = format!("vol/{}/secrets.json", project);
    let path = Path::new(&path_str);
    let rs = random_secrets();
    match fs::read(path.clone()) {
        Ok(data) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => rs,
        },
        Err(_e) => {
            let prefix = path.parent().unwrap();
            fs::create_dir_all(prefix).unwrap();
            put_secrets(path, &rs);
            rs
        }
    }
}

fn get_secrets(project: &str) -> Secrets {
    let path_str = format!("vol/{}/secrets.json", project);
    let path = Path::new(&path_str);
    let data = fs::read(path.clone()).unwrap();
    serde_json::from_slice(&data).unwrap()
}
fn put_secrets(path: &Path, rs: &Secrets) {
    let st = serde_json::to_string_pretty(rs).expect("failed to make json string");
    let mut file = File::create(path).expect("create failed");
    file.write_all(st.as_bytes()).expect("write failed");
}
pub fn add_mnemonic_to_secrets(project: &str, mnemonic: Vec<String>) {
    let mut secrets = get_secrets(project);
    secrets.lnd1_mnemonic = Some(mnemonic);
    let path_str = format!("vol/{}/secrets.json", project);
    let path = Path::new(&path_str);
    put_secrets(path, &secrets);
}
