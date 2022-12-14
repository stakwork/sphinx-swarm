use once_cell::sync::Lazy;
use rocket::*;
use std::collections::HashMap;
use tokio::sync::{Mutex, MutexGuard};

// {tag: {key:value}}
pub type EnvStore = HashMap<String, HashMap<String, String>>;

static ENV: Lazy<Mutex<EnvStore>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn add_to_env(tag: &str, key: &str, value: &str) {
    let env_state = ENV.lock().await;
    add_env(env_state, tag, key, value)
}
fn add_env(mut env_state: MutexGuard<EnvStore>, tag: &str, key: &str, value: &str) {
    if let Some(s) = env_state.get_mut(tag) {
        s.insert(key.to_string(), value.to_string());
    } else {
        let mut nhm = HashMap::new();
        nhm.insert(key.to_string(), value.to_string());
        env_state.insert(tag.to_string(), nhm);
    }
}

pub async fn check_env(tag: &str, txt: &str) -> (String, bool) {
    let env_state = ENV.lock().await;
    let mut ret = txt.to_string();
    let mut skip = false;
    // add to env
    if txt.starts_with("export ") {
        // quoted value
        let mut kv = None;
        if txt.matches("\"").count() == 2 && txt.ends_with("\"") {
            let beg_end: Vec<&str> = txt.split("\"").collect();
            let beg = beg_end[0].split(" ").collect::<Vec<&str>>();
            if let Some(key_eq) = beg.get(1) {
                if key_eq.contains("=") {
                    let key = key_eq.split("=").collect::<Vec<&str>>()[0];
                    let value = beg_end[1];
                    kv = Some((key, value));
                }
            }
        } else {
            // regular hi=asdf
            let txts: Vec<&str> = txt.split(" ").collect();
            if txts.len() > 1 {
                let var = txts.get(1).unwrap();
                let vars: Vec<&str> = var.split("=").collect();
                if vars.len() == 2 {
                    let key = vars.get(0).unwrap();
                    let value = vars.get(1).unwrap();
                    kv = Some((key, value));
                }
            }
        }
        if let Some((key, value)) = kv {
            skip = true;
            add_env(env_state, tag, key, value);
        }
    } else if txt.contains("$") {
        // replace $ env var values
        if let Some(my_vars) = env_state.get(tag) {
            let txts = txt.split(" ");
            let mut ft = "".to_string();
            for t in txts {
                let word = if t.starts_with("$") {
                    let mut key = t.to_string();
                    key.remove(0);
                    if let Some(value) = my_vars.get(&key) {
                        value // replace the var
                    } else {
                        "" // var not found
                    }
                } else {
                    t // regular word
                };
                ft.push_str(&word);
                ft.push_str(" ");
            }
            ret = ft;
        }
    }
    (ret, skip)
}
