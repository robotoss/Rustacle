//! `fs_write` tool: write content to a file.

use async_trait::async_trait;
use serde_json::Value;

use rustacle_llm::types::ToolSchema;

use super::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};

/// Maximum file size for writes (1 MiB).
const MAX_WRITE_SIZE: usize = 1_024 * 1_024;

/// Write or create a file.
pub struct FsWriteTool;

#[async_trait]
impl Tool for FsWriteTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs_write".to_owned(),
            description: "Write content to a file. Creates the file if it doesn't exist, \
                          overwrites if it does."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative path to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        }
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        args.get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .ok_or_else(|| ToolError::Validation("path is required".into()))?;

        let content = args
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("content is required".into()))?;

        if content.len() > MAX_WRITE_SIZE {
            return Err(ToolError::Validation(format!(
                "content too large: {} bytes (max {MAX_WRITE_SIZE})",
                content.len()
            )));
        }

        // Reject binary content
        if content.contains('\0') {
            return Err(ToolError::Validation("binary content not allowed".into()));
        }

        Ok(())
    }

    fn concurrency(&self) -> Concurrency {
        Concurrency::Serialized
    }

    fn required_capabilities(&self, args: &Value) -> Vec<Capability> {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        vec![Capability::Fs { path, write: true }]
    }

    async fn call(&self, args: Value, ctx: ToolCtx) -> Result<ToolOutput, ToolError> {
        let path_str = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        let path = ctx.cwd.join(path_str);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(ToolError::Execution(format!(
                    "parent directory does not exist: {}",
                    parent.display()
                )));
            }
        }

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| ToolError::Execution(format!("write {}: {e}", path.display())))?;

        let lines = content.lines().count();
        let size_kib = content.len() / 1024;
        let summary = format!("wrote {lines} lines ({size_kib} KiB) to {}", path.display());

        Ok(ToolOutput {
            summary,
            payload: None,
        })
    }
}
