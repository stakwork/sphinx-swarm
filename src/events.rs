use rocket::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

pub type EventChans = HashMap<String, broadcast::Sender<String>>;

pub fn new_event_chans() -> EventChans {
    let mut h = HashMap::new();
    h.insert("SWARM".to_string(), broadcast::channel::<String>(1024).0);
    h
}

pub async fn get_event_tx(tag: &str, chans: &Arc<Mutex<EventChans>>) -> broadcast::Sender<String> {
    let chans = chans.lock().await;
    if let Some(sender) = chans.get(tag) {
        return sender.to_owned();
    }
    chans.get("SWARM").unwrap().to_owned()
}
