use crate::{
    cmd::{AddAnthropicKeyReq, SuperSwarmResponse},
    state::Super,
};

pub fn handle_add_anthropic_key(
    state: &mut Super,
    must_save_stack: &mut bool,
    data: AddAnthropicKeyReq,
) -> SuperSwarmResponse {
    if data.key.is_empty() {
        return SuperSwarmResponse {
            success: false,
            message: "Anthropic key cannot be an empty string".to_string(),
            data: None,
        };
    }
    if let Some(_anthropic_keys) = &state.anthropic_keys {
        state.anthropic_keys.as_mut().unwrap().push(data.key);
    } else {
        state.anthropic_keys = Some(vec![data.key])
    }

    *must_save_stack = true;

    SuperSwarmResponse {
        success: true,
        message: "Anthropic key added successfully".to_string(),
        data: None,
    }
}
