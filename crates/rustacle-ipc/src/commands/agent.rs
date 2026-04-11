use serde::{Deserialize, Serialize};

/// Input for `send_prompt`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SendPromptRequest {
    pub message: String,
    pub model_profile: Option<String>,
}

/// Response from `send_prompt`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SendPromptResponse {
    pub turn_id: String,
}
