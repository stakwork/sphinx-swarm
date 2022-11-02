use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;

#[derive(Serialize, Deserialize)]
pub struct Secrets {
    pub bitcoind_pass: String,
    pub lnd1_mnemonic: Vec<String>,
    pub lnd1_password: String,
    pub proxy_admin_token: String,
    pub proxy_store_key: String,
}

const SECRETS_PATH: &str = "/secrets.json";

fn random_secrets() -> Secrets {
    let mnemonic = vec![
        "above", "hair", "trigger", "live", "innocent", "monster", "surprise", "discover", "art",
        "broccoli", "cable", "balcony", "exclude", "maple", "luggage", "dragon", "erosion",
        "basic", "census", "earn", "ripple", "gossip", "record", "monster",
    ];
    Secrets {
        bitcoind_pass: "asdfasdf".to_string(),
        lnd1_mnemonic: mnemonic.iter().map(|s| s.to_string()).collect(),
        lnd1_password: "asdfasdf".to_string(),
        proxy_admin_token: "asdfasdf".to_string(),
        proxy_store_key: "asdfasdf".to_string(),
    }
}

pub fn load_secrets(project: &str) -> Secrets {
    let path = format!("vol/{}/{}.json", project, SECRETS_PATH);
    let rs = random_secrets();
    match fs::read(path.clone()) {
        Ok(data) => match serde_json::from_slice(&data) {
            Ok(d) => d,
            Err(_) => rs,
        },
        Err(_e) => {
            let st = serde_json::to_string_pretty(&rs).expect("failed to make json string");
            let mut file = File::create(path).expect("create failed");
            file.write_all(st.as_bytes()).expect("write failed");
            rs
        }
    }
}

pub fn add_to_secrets() {}
