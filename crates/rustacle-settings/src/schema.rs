//! Typed settings keys with documented default values.
//!
//! Every setting the app supports is an enum variant here.
//! Each variant has a key string, a default value, and a description.

use serde::{Deserialize, Serialize};

/// All known settings keys. New keys can be added with defaults
/// without requiring a migration — unknown keys in the DB are preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingKey {
    // --- Model Profiles ---
    /// Default model profile name.
    DefaultProfile,
    /// JSON array of model profile objects.
    ModelProfiles,

    // --- Agent ---
    /// Max tool calls per turn.
    AgentMaxToolCalls,
    /// Max turn duration in milliseconds.
    AgentMaxDurationMs,
    /// Max total tokens per turn.
    AgentMaxTokens,
    /// Thought flush interval in milliseconds.
    AgentFlushMs,
    /// Memory top-K for prompt assembly.
    AgentMemoryTopK,
    /// Agent role: "developer" or "manager". Affects prompt tone.
    AgentRole,
    /// Max context window tokens. When exceeded, history is compressed.
    /// 0 means use model default.
    AgentMaxContextTokens,

    // --- Terminal ---
    /// Default shell path.
    TerminalShell,
    /// Terminal font size.
    TerminalFontSize,
    /// Terminal font family.
    TerminalFontFamily,

    // --- UI ---
    /// Theme name (e.g., "dark", "light").
    UiTheme,
    /// Keybinding bundle ("vscode", "vim", "emacs").
    UiKeybindings,
    /// Agent panel open by default.
    UiAgentPanelOpen,

    // --- Privacy ---
    /// Whether telemetry is enabled.
    TelemetryEnabled,
}

impl SettingKey {
    /// Database key string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DefaultProfile => "model.default_profile",
            Self::ModelProfiles => "model.profiles",
            Self::AgentMaxToolCalls => "agent.max_tool_calls",
            Self::AgentMaxDurationMs => "agent.max_duration_ms",
            Self::AgentMaxTokens => "agent.max_tokens",
            Self::AgentFlushMs => "agent.flush_ms",
            Self::AgentMemoryTopK => "agent.memory_top_k",
            Self::AgentRole => "agent.role",
            Self::AgentMaxContextTokens => "agent.max_context_tokens",
            Self::TerminalShell => "terminal.shell",
            Self::TerminalFontSize => "terminal.font_size",
            Self::TerminalFontFamily => "terminal.font_family",
            Self::UiTheme => "ui.theme",
            Self::UiKeybindings => "ui.keybindings",
            Self::UiAgentPanelOpen => "ui.agent_panel_open",
            Self::TelemetryEnabled => "privacy.telemetry_enabled",
        }
    }

    /// JSON default value.
    #[must_use]
    pub fn default_json(self) -> serde_json::Value {
        match self {
            Self::DefaultProfile => serde_json::json!("default"),
            Self::ModelProfiles => serde_json::json!([]),
            Self::AgentMaxToolCalls => serde_json::json!(50),
            Self::AgentMaxDurationMs => serde_json::json!(300_000),
            Self::AgentMaxTokens => serde_json::json!(100_000),
            Self::AgentFlushMs => serde_json::json!(80),
            Self::AgentMemoryTopK => serde_json::json!(6),
            Self::AgentRole => serde_json::json!("developer"),
            Self::AgentMaxContextTokens => serde_json::json!(0),
            Self::TerminalShell => serde_json::json!(""),
            Self::TerminalFontSize => serde_json::json!(14),
            Self::TerminalFontFamily => serde_json::json!("monospace"),
            Self::UiTheme => serde_json::json!("dark"),
            Self::UiKeybindings => serde_json::json!("vscode"),
            Self::UiAgentPanelOpen | Self::TelemetryEnabled => serde_json::json!(false),
        }
    }

    /// Human-readable description.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::DefaultProfile => "Default model profile name",
            Self::ModelProfiles => "List of configured model profiles",
            Self::AgentMaxToolCalls => "Maximum tool calls per agent turn",
            Self::AgentMaxDurationMs => "Maximum turn duration (ms)",
            Self::AgentMaxTokens => "Maximum total tokens per turn",
            Self::AgentFlushMs => "Thought flush interval (ms)",
            Self::AgentMemoryTopK => "Number of memory entries in prompt",
            Self::AgentRole => {
                "Agent role (developer/manager/blogger/analyst/devops/designer/student)"
            }
            Self::AgentMaxContextTokens => {
                "Max context window tokens (0 = model default, history compressed when exceeded)"
            }
            Self::TerminalShell => "Default shell path (empty = auto-detect)",
            Self::TerminalFontSize => "Terminal font size in pixels",
            Self::TerminalFontFamily => "Terminal font family",
            Self::UiTheme => "UI color theme",
            Self::UiKeybindings => "Keybinding bundle (vscode/vim/emacs)",
            Self::UiAgentPanelOpen => "Agent panel open by default",
            Self::TelemetryEnabled => "Anonymous telemetry enabled",
        }
    }

    /// Look up a key by its string representation.
    #[must_use]
    pub fn from_key_str(s: &str) -> Option<Self> {
        Self::ALL.iter().find(|k| k.as_str() == s).copied()
    }

    /// All known keys (for iteration).
    pub const ALL: &'static [SettingKey] = &[
        Self::DefaultProfile,
        Self::ModelProfiles,
        Self::AgentMaxToolCalls,
        Self::AgentMaxDurationMs,
        Self::AgentMaxTokens,
        Self::AgentFlushMs,
        Self::AgentMemoryTopK,
        Self::AgentRole,
        Self::AgentMaxContextTokens,
        Self::TerminalShell,
        Self::TerminalFontSize,
        Self::TerminalFontFamily,
        Self::UiTheme,
        Self::UiKeybindings,
        Self::UiAgentPanelOpen,
        Self::TelemetryEnabled,
    ];
}

/// A setting entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub description: String,
}
