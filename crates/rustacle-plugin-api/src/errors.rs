use serde::{Deserialize, Serialize};

/// Errors originating from a plugin or the plugin host.
#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", content = "data")]
pub enum ModuleError {
    /// The plugin was denied a capability.
    #[error("permission denied: {capability}")]
    Denied { capability: String },

    /// The plugin received invalid input.
    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },

    /// The WASM guest trapped (fuel exhaustion, unreachable, etc.).
    #[error("wasm trap: {0}")]
    Trap(String),

    /// Unrecoverable internal error.
    #[error("internal: {0}")]
    Internal(String),
}
