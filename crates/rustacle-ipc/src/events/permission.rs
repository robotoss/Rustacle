use serde::{Deserialize, Serialize};

/// Permission request from the kernel to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PermissionAskEvent {
    pub request_id: String,
    pub plugin_id: String,
    pub capability: String,
    pub description: String,
}

/// User's response to a permission request.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PermissionResponseEvent {
    pub request_id: String,
    pub granted: bool,
}
