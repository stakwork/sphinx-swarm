use once_cell::sync::Lazy;
use rocket::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

// {tag: [log]}
pub type LogStore = HashMap<String, Vec<String>>;

pub static LOGS: Lazy<Mutex<LogStore>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub type LogChans = HashMap<String, broadcast::Sender<String>>;

pub fn new_log_chans() -> LogChans {
    let mut h = HashMap::new();
    h.insert("".to_string(), broadcast::channel::<String>(1024).0);
    h
}

pub async fn get_log_tx(tag: &str, chans: &Arc<Mutex<LogChans>>) -> broadcast::Sender<String> {
    let chans = chans.lock().await;
    if let Some(sender) = chans.get(tag) {
        return sender.to_owned();
    }
    chans.get("").unwrap().to_owned()
}

async fn add_log(tag: String, text: String) {
    let mut lgs = LOGS.lock().await;
    if let Some(inner) = lgs.get_mut(&tag) {
        inner.push(text);
    } else {
        lgs.insert(tag, vec![text]);
    }
}

pub fn collect_logs(tag: &str, log_tx: broadcast::Sender<String>) {
    let mut stream = log_tx.subscribe();
    let tag = tag.to_string();
    tokio::spawn(async move {
        while let Ok(lo) = stream.recv().await {
            add_log(tag.clone(), lo).await;
        }
    });
}
