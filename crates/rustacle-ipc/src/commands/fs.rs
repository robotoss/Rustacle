use serde::{Deserialize, Serialize};

/// Request to read a file.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ReadFileRequest {
    pub path: String,
}

/// Response from `read_file`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ReadFileResponse {
    pub content: String,
    pub size: u64,
}

/// A directory entry.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}
