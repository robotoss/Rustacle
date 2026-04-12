use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::Kernel;
use crate::registry::PluginRegistry;

/// Application state managed by Tauri.
pub struct AppState {
    pub kernel: Arc<Kernel>,
    pub registry: Arc<PluginRegistry>,
    pub agent_session: Arc<Mutex<AgentSession>>,
    pub settings: Arc<rustacle_settings::SettingsStore>,
}

/// Tracks the active agent turn and conversation history.
#[derive(Default)]
pub struct AgentSession {
    /// Cancel handles keyed by `turn_id`.
    pub active_cancels: HashMap<String, tokio_util::sync::CancellationToken>,
    /// Conversation history for prompt assembly.
    pub history: Vec<AgentHistoryEntry>,
}

/// A completed turn in conversation history.
pub struct AgentHistoryEntry {
    pub user_message: String,
    pub assistant_answer: String,
}
