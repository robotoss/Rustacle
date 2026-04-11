//! `fs_edit` tool: string-replace edit with uniqueness check.

use async_trait::async_trait;
use serde_json::Value;

use rustacle_llm::types::ToolSchema;

use super::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};

/// Edit a file by replacing an exact string match.
pub struct FsEditTool;

#[async_trait]
impl Tool for FsEditTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs_edit".to_owned(),
            description: "Edit a file by replacing an exact string match. The old_string \
                          must appear exactly once in the file (unless replace_all is true)."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact text to find and replace (must be unique)"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement text"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false)"
                    }
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": false
            }),
        }
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        args.get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .ok_or_else(|| ToolError::Validation("path is required".into()))?;

        let old = args
            .get("old_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("old_string is required".into()))?;

        if old.is_empty() {
            return Err(ToolError::Validation("old_string cannot be empty".into()));
        }

        args.get("new_string")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("new_string is required".into()))?;

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
        let old_string = args["old_string"].as_str().unwrap_or("");
        let new_string = args["new_string"].as_str().unwrap_or("");
        let replace_all = args
            .get("replace_all")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let path = ctx.cwd.join(path_str);

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Execution(format!("read {}: {e}", path.display())))?;

        let count = content.matches(old_string).count();

        if count == 0 {
            return Err(ToolError::Execution(format!(
                "old_string not found in {}",
                path.display()
            )));
        }

        if count > 1 && !replace_all {
            return Err(ToolError::Execution(format!(
                "old_string found {count} times in {} (use replace_all=true for multiple)",
                path.display()
            )));
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        tokio::fs::write(&path, &new_content)
            .await
            .map_err(|e| ToolError::Execution(format!("write {}: {e}", path.display())))?;

        // Compute line diff
        let old_lines = old_string.lines().count();
        let new_lines = new_string.lines().count();
        let added = new_lines.saturating_sub(old_lines);
        let removed = old_lines.saturating_sub(new_lines);

        let summary = format!("+{added} -{removed} lines in {}", path.display());

        Ok(ToolOutput {
            summary,
            payload: None,
        })
    }
}
