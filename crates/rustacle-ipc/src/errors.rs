use serde::{Deserialize, Serialize};

/// Typed error enum for all IPC commands.
/// Serialized as externally tagged JSON for exhaustive TS matching.
#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", content = "data")]
pub enum RustacleError {
    #[error("not found: {resource}")]
    NotFound { resource: String },

    #[error("denied: {action}: {reason}")]
    Denied { action: String, reason: String },

    #[error("invalid input: {field}: {message}")]
    InvalidInput { field: String, message: String },

    #[error("internal: {message}")]
    Internal { message: String },

    #[error("plugin error: {plugin_id}: {message}")]
    PluginError { plugin_id: String, message: String },
}
