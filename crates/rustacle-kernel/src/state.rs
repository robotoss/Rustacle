use std::sync::Arc;

use crate::Kernel;
use crate::registry::PluginRegistry;

/// Application state managed by Tauri.
pub struct AppState {
    pub kernel: Arc<Kernel>,
    pub registry: Arc<PluginRegistry>,
    // Future: settings, bus, llm, permission
}
