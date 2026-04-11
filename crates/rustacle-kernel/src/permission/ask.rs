use rustacle_plugin_api::{Capability, ModuleError};
use tokio::sync::oneshot;

/// A permission request sent to the UI for user approval.
pub struct PermissionAsk {
    pub plugin_id: String,
    pub capability: Capability,
    pub reply_tx: oneshot::Sender<PermissionDecision>,
}

/// The user's decision about a capability request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Allow for this session and cache.
    Allow,
    /// Allow only for the current session (cleared on restart).
    AllowSession,
    /// Deny this request (not cached — user can retry).
    Deny,
}

/// A cached grant entry.
#[derive(Debug, Clone)]
pub struct Grant {
    pub decision: PermissionDecision,
}

impl Grant {
    /// Convert this grant into a `Result` suitable for plugin code.
    ///
    /// # Errors
    /// Returns `ModuleError::Denied` if the grant is `Deny`.
    pub fn as_result(&self, cap: &Capability) -> Result<(), ModuleError> {
        match self.decision {
            PermissionDecision::Allow | PermissionDecision::AllowSession => Ok(()),
            PermissionDecision::Deny => Err(ModuleError::Denied {
                capability: format!("{cap:?}"),
            }),
        }
    }
}
