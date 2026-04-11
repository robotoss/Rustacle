use serde::{Deserialize, Serialize};

/// Response from the `ping` command.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PingResponse {
    pub message: String,
    pub timestamp: u64,
}
