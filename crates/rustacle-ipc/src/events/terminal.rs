use serde::{Deserialize, Serialize};

use crate::commands::terminal::SplitLayout;

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

/// A new tab was opened.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TabOpenedEvent {
    pub tab_id: String,
    pub index: u32,
}

/// A tab was closed.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TabClosedEvent {
    pub tab_id: String,
}

/// The split layout changed (split, close, resize).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct LayoutChangedEvent {
    pub layout: Option<SplitLayout>,
}

/// A tab's title changed.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TabTitleChangedEvent {
    pub tab_id: String,
    pub title: String,
}
