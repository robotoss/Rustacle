use serde::{Deserialize, Serialize};

/// Agent interaction mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum AgentMode {
    /// Full tools enabled, `ReAct` loop.
    #[default]
    Chat,
    /// Read-only tools only, planning overlay.
    Plan,
    /// No tools, direct Q&A.
    Ask,
}

/// Input for `send_prompt`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SendPromptRequest {
    pub message: String,
    pub model_profile: Option<String>,
    pub mode: AgentMode,
}

/// Response from `send_prompt`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SendPromptResponse {
    pub turn_id: String,
}

/// Input for `stop_turn`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct StopTurnRequest {
    pub turn_id: String,
}

/// Response from `stop_turn`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct StopTurnResponse {
    pub cancelled: bool,
}

/// Input for `respond_permission`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct RespondPermissionRequest {
    pub turn_id: String,
    pub step_id: String,
    pub decision: PermissionDecision,
}

/// Permission decision from the user.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
pub enum PermissionDecision {
    Deny,
    AllowOnce,
    AllowAlways,
}

/// Summary of a model profile for the quick-switcher.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ProfileSummary {
    pub name: String,
    pub provider: String,
    pub model: String,
}

/// Response from `list_model_profiles`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ListModelProfilesResponse {
    pub profiles: Vec<ProfileSummary>,
}
