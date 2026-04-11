use std::collections::BTreeMap;
use std::path::PathBuf;

use rustacle_llm::types::ModelProfile;

/// Unique turn identifier.
pub type TurnId = String;

/// Tool identifier (e.g., `fs_read`, `grep`).
pub type ToolId = String;

/// Unix timestamp in milliseconds, injected for determinism.
pub type UnixMillis = u64;

/// A user message that starts or continues a turn.
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub text: String,
}

/// Conversation history: ordered list of past messages.
#[derive(Debug, Clone, Default)]
pub struct ConversationHistory {
    pub messages: Vec<HistoryMessage>,
}

/// A single message in conversation history.
#[derive(Debug, Clone)]
pub struct HistoryMessage {
    pub role: HistoryRole,
    pub content: String,
}

/// Role for history messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryRole {
    User,
    Assistant,
    Tool,
}

/// Snapshot of a terminal tab at turn start.
#[derive(Debug, Clone)]
pub struct TabSnapshot {
    pub index: usize,
    pub title: String,
    pub cwd: PathBuf,
    pub shell_name: String,
    pub shell_path: String,
    pub last_commands: Vec<CommandRecord>,
}

/// A previously-run command in a tab.
#[derive(Debug, Clone)]
pub struct CommandRecord {
    pub command: String,
    pub exit_code: i32,
}

/// Summary of a tab for the tabs list.
#[derive(Debug, Clone)]
pub struct TabSummary {
    pub index: usize,
    pub title: String,
    pub cwd: PathBuf,
    pub shell_name: String,
    pub last_cmd: Option<CommandRecord>,
}

/// Host OS information.
#[derive(Debug, Clone)]
pub struct HostOs {
    pub name: String,
    pub version: String,
}

/// What permissions are currently granted.
#[derive(Debug, Clone, Default)]
pub struct PermissionView {
    /// Tools that have been granted permission.
    pub granted_tools: Vec<ToolId>,
}

impl PermissionView {
    /// Check if a tool has permission.
    #[must_use]
    pub fn allowed_for_tool(&self, tool_id: &str) -> bool {
        self.granted_tools.iter().any(|t| t == tool_id)
    }
}

/// Project documentation files found walking up from cwd.
#[derive(Debug, Clone, Default)]
pub struct ProjectDocs {
    /// Docs ordered from outermost (repo root) to innermost (closest to cwd).
    pub docs: Vec<ProjectDoc>,
}

/// A single project documentation file.
#[derive(Debug, Clone)]
pub struct ProjectDoc {
    pub rel_path: String,
    pub body: String,
}

/// Memory entries available for the turn.
#[derive(Debug, Clone, Default)]
pub struct MemoryView {
    /// All memory entries, pre-scored. Sorted by score descending.
    pub entries: Vec<MemoryEntry>,
}

/// A scored memory entry.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub text: String,
    pub score: f64,
}

impl MemoryView {
    /// Return top-K entries relevant to the user turn.
    /// Entries are pre-scored externally; we just take the top K.
    #[must_use]
    pub fn top_k(&self, k: usize) -> &[MemoryEntry] {
        let end = k.min(self.entries.len());
        &self.entries[..end]
    }
}

/// Selected (pinned) file with its content.
#[derive(Debug, Clone)]
pub struct SelectedFile {
    pub path: PathBuf,
    pub content: String,
    pub language: String,
}

/// Complete context for a single agent turn.
///
/// All fields are snapshots captured at turn start. Determinism requires
/// that no field changes during prompt assembly.
pub struct TurnContext {
    pub turn_id: TurnId,
    pub user_turn: UserMessage,
    pub history: ConversationHistory,

    // UI state at turn start
    pub model_profile: ModelProfile,
    pub ui_enabled_tools: Vec<ToolId>,
    pub active_tab: TabSnapshot,
    pub open_tabs: Vec<TabSummary>,
    pub host_os: HostOs,

    // Plugin services (pre-fetched snapshots)
    pub permissions: PermissionView,
    pub project_docs: ProjectDocs,
    pub memory: MemoryView,
    pub selected_files: Vec<SelectedFile>,

    /// Injected clock for deterministic tests.
    pub now: UnixMillis,
    /// Timezone name for display (e.g., "Europe/Moscow").
    pub timezone: String,

    /// Extra per-profile data, sorted for determinism.
    pub extra: BTreeMap<String, String>,
}
