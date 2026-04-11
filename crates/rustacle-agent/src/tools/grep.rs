//! `grep` tool: pattern search using regex (ripgrep-style).

use async_trait::async_trait;
use serde_json::Value;

use rustacle_llm::types::ToolSchema;

use super::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};

/// Maximum matches to return.
const MAX_MATCHES: usize = 500;

/// Search file contents with a regex pattern.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "grep".to_owned(),
            description: "Search file contents with a regex pattern. Returns matching \
                          lines with file paths and line numbers."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file to search in"
                    },
                    "glob": {
                        "type": "string",
                        "description": "File glob filter (e.g., \"*.rs\")"
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
        let search_path = args.get("path").and_then(Value::as_str).unwrap_or(".");
        let path = ctx.cwd.join(search_path);

        // Simple regex-based grep implementation
        // In production, this would use the `grep` crate (ripgrep library)
        let regex = regex_lite::Regex::new(pattern)
            .map_err(|e| ToolError::Validation(format!("invalid regex: {e}")))?;

        let mut matches = Vec::new();
        let mut file_count = 0u32;

        if path.is_file() {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                let file_matches = grep_file(&regex, &path, &content, MAX_MATCHES);
                if !file_matches.is_empty() {
                    file_count += 1;
                }
                matches.extend(file_matches);
            }
        } else if path.is_dir() {
            let mut entries = Vec::new();
            collect_files(&path, &mut entries, 10).await;

            let glob_filter = args.get("glob").and_then(Value::as_str);

            for entry in entries {
                if matches.len() >= MAX_MATCHES {
                    break;
                }

                // Apply glob filter
                if let Some(glob_pat) = glob_filter {
                    let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !simple_glob_match(glob_pat, name) {
                        continue;
                    }
                }

                if let Ok(content) = tokio::fs::read_to_string(&entry).await {
                    let remaining = MAX_MATCHES - matches.len();
                    let file_matches = grep_file(&regex, &entry, &content, remaining);
                    if !file_matches.is_empty() {
                        file_count += 1;
                    }
                    matches.extend(file_matches);
                }
            }
        }

        let total = matches.len();
        let body = matches.join("\n");
        let summary = format!("{total} matches in {file_count} files");

        Ok(ToolOutput {
            summary,
            payload: Some(bytes::Bytes::from(body)),
        })
    }
}

fn grep_file(
    regex: &regex_lite::Regex,
    path: &std::path::Path,
    content: &str,
    max: usize,
) -> Vec<String> {
    let mut results = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if results.len() >= max {
            break;
        }
        if regex.is_match(line) {
            results.push(format!("{}:{}:{}", path.display(), i + 1, line));
        }
    }
    results
}

/// Recursively collect files up to a depth limit.
async fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>, max_depth: u32) {
    if max_depth == 0 {
        return;
    }
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            out.push(path);
        } else if path.is_dir() {
            // Skip hidden directories
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') {
                Box::pin(collect_files(&path, out, max_depth - 1)).await;
            }
        }
    }
}

/// Simple glob matching (just `*.ext` patterns).
fn simple_glob_match(pattern: &str, name: &str) -> bool {
    if let Some(ext) = pattern.strip_prefix("*.") {
        name.ends_with(&format!(".{ext}"))
    } else {
        name == pattern
    }
}
