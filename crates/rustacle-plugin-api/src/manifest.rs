use serde::{Deserialize, Serialize};

use crate::Capability;

/// Declares a plugin's identity, requirements, and UI contributions.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ModuleManifest {
    /// Unique plugin identifier (e.g. `"rustacle.fs"`).
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Semantic version.
    pub version: String,

    /// Capabilities this plugin requires from the host.
    pub capabilities: Vec<Capability>,

    /// Event bus topics this plugin subscribes to.
    pub subscriptions: Vec<String>,

    /// UI surfaces contributed by this plugin.
    pub ui_contributions: UiContributions,
}

/// UI surfaces a plugin contributes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct UiContributions {
    /// Side panels contributed to the UI.
    pub panels: Vec<PanelDesc>,

    /// Command palette entries.
    pub palette_commands: Vec<PaletteEntry>,

    /// JSON Schema for plugin-specific settings (rendered by Settings UI).
    pub settings_schema: Option<String>,
}

/// Description of a UI panel contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PanelDesc {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
}

/// A command palette entry contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PaletteEntry {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
}
