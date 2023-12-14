use rocket::*;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

pub type EventChan = broadcast::Sender<String>;

pub fn new_event_chan() -> Arc<Mutex<EventChan>> {
    Arc::new(Mutex::new(broadcast::channel::<String>(1024).0))
}

pub async fn get_event_tx(chan: &Arc<Mutex<EventChan>>) -> broadcast::Sender<String> {
    chan.lock().await.clone()
}
