use serde::{Deserialize, Serialize};

/// Request to open a new terminal tab.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct OpenTabRequest {
    pub cwd: Option<String>,
}

/// Response from `open_tab`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct OpenTabResponse {
    pub tab_id: String,
}
