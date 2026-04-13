use serde::{Deserialize, Serialize};

/// Request to open a new terminal tab.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct OpenTabRequest {
    pub cwd: Option<String>,
}

/// Response from `open_tab`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct OpenTabResponse {
    pub tab_id: String,
}

/// Request to split a tab into two panes.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SplitTabRequest {
    pub tab_id: String,
    /// `"horizontal"` or `"vertical"`.
    pub direction: String,
}

/// Response from `split_tab`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SplitTabResponse {
    pub new_tab_id: String,
    pub split_node_id: String,
}

/// Request to resize a split divider.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ResizeSplitRequest {
    pub node_id: String,
    pub ratio: f64,
}

/// Request to reorder a tab.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ReorderTabRequest {
    pub tab_id: String,
    pub new_index: u32,
}

/// Serializable tab info.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TabInfo {
    pub id: String,
    pub cwd: String,
    pub title: String,
    pub alive: bool,
    pub index: u32,
    pub active: bool,
}

/// A node in the recursive split layout tree.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SplitLayout {
    Leaf {
        tab_id: String,
    },
    Split {
        id: String,
        direction: String,
        ratio: f64,
        children: Vec<SplitLayout>,
    },
}

/// Per-tab agent context returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TabContextResponse {
    pub cwd: String,
    pub last_commands: Vec<CommandRecord>,
}

/// A single command record from tab history.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CommandRecord {
    pub command: String,
    pub exit_code: i32,
    pub timestamp_ms: u64,
}
