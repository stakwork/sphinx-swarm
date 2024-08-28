use crate::state::{RemoteStack, Super};
use std::collections::HashMap;

pub fn add_new_swarm_details(
    state: &mut Super,
    swarm_details: RemoteStack,
    must_save_stack: &mut bool,
) -> HashMap<String, String> {
    let mut hm = HashMap::new();
    match state.find_swarm_by_host(&swarm_details.host) {
        Some(_swarm) => {
            hm.insert("success".to_string(), "false".to_string());
            hm.insert("message".to_string(), "swarm already exist".to_string());
        }
        None => {
            state.add_remote_stack(swarm_details);
            *must_save_stack = true;
            hm.insert("success".to_string(), "true".to_string());
            hm.insert(
                "message".to_string(),
                "Swarm added successfully".to_string(),
            );
        }
    }
    hm
}
