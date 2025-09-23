use crate::{cmd::SuperSwarmResponse, state::Super};

pub fn handle_get_anthropic_keys(state: &Super) -> SuperSwarmResponse {
    let mut data: Vec<String> = vec![];
    if let Some(anthropic_keys) = &state.anthropic_keys {
        data = anthropic_keys.clone();
    }

    let anthropic_keys = match serde_json::to_value(data) {
        Ok(json) => json,
        Err(err) => {
            log::error!(
                "Error converting vec (anthropic keys) to Value: {}",
                err.to_string()
            );
            return SuperSwarmResponse {
                success: false,
                message: format!(
                    "Error converting vec (anthropic keys) to Value: {}",
                    err.to_string()
                ),
                data: None,
            };
        }
    };

    SuperSwarmResponse {
        success: true,
        message: "Anthropic keys successfully".to_string(),
        data: Some(anthropic_keys),
    }
}
