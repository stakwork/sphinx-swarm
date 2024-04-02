use crate::secrets;
use once_cell::sync::Lazy;
use rocket::tokio::sync::Mutex;
use std::collections::HashMap;

pub static DETAILS: Lazy<Mutex<HashMap<String, HashMap<u32, String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn generate_signup_challenge(user_id: u32) -> String {
    let challenge = secrets::random_word(16);
    let mut user_details: HashMap<u32, String> = HashMap::new();

    user_details.insert(user_id, "".to_string());
    let mut details = DETAILS.lock().await;
    details.insert(challenge.clone(), user_details);
    challenge.to_string()
}
