use serde::{Deserialize, Serialize};

/// A single setting entry.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SettingEntry {
    pub key: String,
    pub value: serde_json::Value,
}
