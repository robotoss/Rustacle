//! `fs_read` tool: read file contents with offset/limit paging.

use async_trait::async_trait;
use serde_json::Value;

use rustacle_llm::types::ToolSchema;

use super::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};

/// Read a file's contents.
pub struct FsReadTool;

#[async_trait]
impl Tool for FsReadTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs_read".to_owned(),
            description: "Read the contents of a file. Returns the file text, \
                          or an error if the path is outside the granted scope."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative path to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start reading from (0-based)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to read"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        args.get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .ok_or_else(|| ToolError::Validation("path is required".into()))?;
        Ok(())
    }

    fn concurrency(&self) -> Concurrency {
        Concurrency::Concurrent
    }

    fn required_capabilities(&self, args: &Value) -> Vec<Capability> {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        vec![Capability::Fs {
            path,
            write: false,
        }]
    }

    async fn call(&self, args: Value, ctx: ToolCtx) -> Result<ToolOutput, ToolError> {
        let path_str = args["path"].as_str().unwrap_or("");
        let path = ctx.cwd.join(path_str);

        #[allow(clippy::cast_possible_truncation)]
        let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        #[allow(clippy::cast_possible_truncation)]
        let limit = args.get("limit").and_then(Value::as_u64).map(|l| l as usize);

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::Execution(format!("read {}: {e}", path.display())))?;

        // Check for binary content (simple heuristic)
        if content.contains('\0') {
            return Ok(ToolOutput {
                summary: format!("binary file: {} ({} bytes)", path.display(), content.len()),
                payload: None,
            });
        }

        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let start = offset.min(total);
        let end = limit.map_or(total, |l| (start + l).min(total));
        let selected = &lines[start..end];
        let body = selected.join("\n");

        let summary = format!(
            "read {} lines {}-{} of {} from {}",
            end - start,
            start,
            end,
            total,
            path.display()
        );

        Ok(ToolOutput {
            summary,
            payload: Some(bytes::Bytes::from(body)),
        })
    }
}
