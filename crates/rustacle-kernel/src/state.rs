use std::sync::Arc;

use crate::Kernel;

/// Application state managed by Tauri.
pub struct AppState {
    pub kernel: Arc<Kernel>,
    // Future: settings, bus, llm, permission
}
