//! `bash` tool: execute a shell command via the terminal plugin.

use async_trait::async_trait;
use serde_json::Value;

use rustacle_llm::types::ToolSchema;

use super::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};

/// Dangerous command patterns that require explicit warning.
const DESTRUCTIVE_PATTERNS: &[&str] = &[
    "rm -rf",
    "rm -r",
    "rmdir",
    "git push --force",
    "git push -f",
    "git reset --hard",
    "DROP TABLE",
    "DROP DATABASE",
    "truncate",
    "format",
];

/// Commands that are never allowed.
const BLOCKED_PATTERNS: &[&str] = &[
    "sed -i",  // Use fs_edit instead
];

/// Execute a shell command.
pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "bash".to_owned(),
            description: "Execute a shell command in the target terminal tab. \
                          Returns stdout, stderr, and exit code."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "tab_target": {
                        "type": "integer",
                        "description": "Tab index to run in (default: active tab)"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in milliseconds (default: 120000)"
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn prompt_addendum(&self) -> &'static str {
        "Prefer dedicated tools (fs_read, fs_write, fs_edit, grep, glob) over bash \
         when a dedicated tool exists. Use bash only for commands that have no \
         dedicated tool equivalent."
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        let command = args
            .get("command")
            .and_then(Value::as_str)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| ToolError::Validation("command is required".into()))?;

        // Block sed -i (use fs_edit instead)
        for pattern in BLOCKED_PATTERNS {
            if command.contains(pattern) {
                return Err(ToolError::Validation(format!(
                    "'{pattern}' is blocked — use the dedicated tool instead"
                )));
            }
        }

        // Warn about destructive commands (validation still passes, but the
        // harness will see the warning in the ToolCall step)
        for pattern in DESTRUCTIVE_PATTERNS {
            if command.contains(pattern) {
                tracing::warn!(
                    command,
                    pattern,
                    "destructive command detected — permission check required"
                );
                break;
            }
        }

        Ok(())
    }

    fn concurrency(&self) -> Concurrency {
        Concurrency::Serialized
    }

    fn required_capabilities(&self, _args: &Value) -> Vec<Capability> {
        vec![Capability::Pty]
    }

    async fn call(&self, args: Value, ctx: ToolCtx) -> Result<ToolOutput, ToolError> {
        let command = args["command"].as_str().unwrap_or("");
        let timeout_ms = args
            .get("timeout")
            .and_then(Value::as_u64)
            .unwrap_or(120_000);

        // In the full system, this delegates to the terminal plugin via
        // a kernel-mediated command. For now, we execute directly.
        let start = std::time::Instant::now();

        let output = tokio::select! {
            result = tokio::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .current_dir(&ctx.cwd)
                .output() => {
                result.map_err(|e| ToolError::Execution(format!("spawn: {e}")))?
            }
            () = tokio::time::sleep(std::time::Duration::from_millis(timeout_ms)) => {
                return Err(ToolError::Execution(format!(
                    "command timed out after {timeout_ms}ms"
                )));
            }
            () = ctx.cancel.cancelled() => {
                return Err(ToolError::Cancelled);
            }
        };

        let elapsed = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);
        let total_bytes = stdout.len() + stderr.len();

        let summary = format!(
            "exit {} in {:.1}s, {} bytes output",
            exit_code,
            elapsed.as_secs_f64(),
            total_bytes
        );

        let body = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{stdout}\n--- stderr ---\n{stderr}")
        };

        Ok(ToolOutput {
            summary,
            payload: Some(bytes::Bytes::from(body)),
        })
    }
}
