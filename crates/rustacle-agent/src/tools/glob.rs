//! `glob` tool: find files matching a glob pattern.

use async_trait::async_trait;
use serde_json::Value;

use rustacle_llm::types::ToolSchema;

use super::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};

/// Maximum results to return.
const MAX_RESULTS: usize = 1000;

/// Find files matching a glob pattern.
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "glob".to_owned(),
            description: "Find files matching a glob pattern. Returns a list of \
                          matching file paths sorted by modification time."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., \"src/**/*.rs\")"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory to search from"
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        }
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        args.get("pattern")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .ok_or_else(|| ToolError::Validation("pattern is required".into()))?;
        Ok(())
    }

    fn concurrency(&self) -> Concurrency {
        Concurrency::Concurrent
    }

    fn required_capabilities(&self, args: &Value) -> Vec<Capability> {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_owned();
        vec![Capability::Fs { path, write: false }]
    }

    async fn call(&self, args: Value, ctx: ToolCtx) -> Result<ToolOutput, ToolError> {
        let pattern = args["pattern"].as_str().unwrap_or("");
        let base = args.get("path").and_then(Value::as_str).unwrap_or(".");
        let base_path = ctx.cwd.join(base);

        let full_pattern = base_path.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let paths: Vec<std::path::PathBuf> = glob::glob(&pattern_str)
            .map_err(|e| ToolError::Execution(format!("invalid glob: {e}")))?
            .filter_map(Result::ok)
            .take(MAX_RESULTS)
            .collect();

        // Sort by mtime (newest first)
        let mut with_mtime: Vec<(std::path::PathBuf, u64)> = paths
            .into_iter()
            .map(|p| {
                let mtime = p
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                (p, mtime)
            })
            .collect();
        with_mtime.sort_by(|a, b| b.1.cmp(&a.1));

        let total = with_mtime.len();
        let body = with_mtime
            .iter()
            .map(|(p, _): &(std::path::PathBuf, u64)| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let summary = format!("found {total} files (showing {total})");

        Ok(ToolOutput {
            summary,
            payload: Some(bytes::Bytes::from(body)),
        })
    }
}
