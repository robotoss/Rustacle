use serde::{Deserialize, Serialize};

/// Summary of a loaded plugin.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PluginSummary {
    pub id: String,
    pub version: String,
    pub state: PluginState,
}

/// Plugin lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum PluginState {
    Loading,
    Running,
    Suspended,
    Error,
}

/// Response from `list_plugins`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ListPluginsResponse {
    pub plugins: Vec<PluginSummary>,
}
