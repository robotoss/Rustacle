use crate::bindings::rustacle::plugin::host;
use crate::bindings::rustacle::plugin::types::ModuleError;
use serde::{Deserialize, Serialize};

/// Request to read a file.
#[derive(Deserialize)]
struct ReadFileRequest {
    path: String,
    #[serde(default)]
    start_line: Option<u32>,
    #[serde(default)]
    end_line: Option<u32>,
}

/// Response from reading a file.
#[derive(Serialize)]
struct ReadFileResponse {
    content: String,
    total_lines: u32,
}

/// Read a file via the host `fs-read` function (permission-gated).
pub fn read_file(payload: &[u8]) -> Result<Vec<u8>, ModuleError> {
    let req: ReadFileRequest = serde_json::from_slice(payload)
        .map_err(|e| ModuleError::InvalidInput(format!("bad json: {e}")))?;

    let bytes = host::fs_read(&req.path)
        .map_err(|e| ModuleError::Internal(format!("fs-read failed: {e:?}")))?;

    let full = String::from_utf8(bytes)
        .map_err(|_| ModuleError::InvalidInput("file is not valid UTF-8".to_string()))?;

    let lines: Vec<&str> = full.lines().collect();
    let total_lines = lines.len() as u32;

    let start = req.start_line.unwrap_or(1).saturating_sub(1) as usize;
    let end = req.end_line.map_or(lines.len(), |e| e as usize).min(lines.len());

    let content = lines
        .get(start..end)
        .map(|s| s.join("\n"))
        .unwrap_or_default();

    let resp = ReadFileResponse {
        content,
        total_lines,
    };

    serde_json::to_vec(&resp).map_err(|e| ModuleError::Internal(format!("serialize: {e}")))
}

/// List a directory via the host `fs-read` function.
/// Uses a convention: reading a directory path returns JSON entries.
pub fn list_dir(payload: &[u8]) -> Result<Vec<u8>, ModuleError> {
    let req: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| ModuleError::InvalidInput(format!("bad json: {e}")))?;

    let path = req
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ModuleError::InvalidInput("missing 'path' field".to_string()))?;

    // For now, delegate to host fs-read; actual directory listing
    // will be a dedicated host function in a future WIT version.
    let bytes = host::fs_read(path)
        .map_err(|e| ModuleError::Internal(format!("fs-read failed: {e:?}")))?;

    // Pass through host response
    Ok(bytes)
}

/// File stat information.
#[derive(Serialize)]
struct StatResponse {
    path: String,
    size: u64,
    is_dir: bool,
}

/// Get file metadata via host.
pub fn stat(payload: &[u8]) -> Result<Vec<u8>, ModuleError> {
    let req: serde_json::Value = serde_json::from_slice(payload)
        .map_err(|e| ModuleError::InvalidInput(format!("bad json: {e}")))?;

    let path = req
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ModuleError::InvalidInput("missing 'path' field".to_string()))?;

    // Stub: real implementation reads metadata via a host function.
    let resp = StatResponse {
        path: path.to_string(),
        size: 0,
        is_dir: false,
    };

    serde_json::to_vec(&resp).map_err(|e| ModuleError::Internal(format!("serialize: {e}")))
}
