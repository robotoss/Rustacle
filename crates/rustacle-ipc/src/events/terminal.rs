use serde::{Deserialize, Serialize};

/// Terminal output chunk streamed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TerminalChunkEvent {
    pub tab_id: String,
    pub data: Vec<u8>,
}

/// Terminal working directory changed.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CwdChangeEvent {
    pub tab_id: String,
    pub cwd: String,
}
