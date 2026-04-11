//! Tool schema collection and rendering for the provider's tool-use dialect.
//!
//! Tool schemas go through the provider's native `tools` parameter, never
//! into free-text prose. This module filters tools by UI-enabled flag and
//! permission grants, then collects their schemas.

use rustacle_llm::types::ToolSchema;

use crate::turn_context::TurnContext;

/// Collect tool schemas for tools that are both UI-enabled and permission-granted.
///
/// Returns schemas sorted by tool name for determinism.
#[must_use]
pub fn collect_tool_schemas(ctx: &TurnContext) -> Vec<ToolSchema> {
    let mut schemas: Vec<ToolSchema> = ctx
        .ui_enabled_tools
        .iter()
        .filter(|id| ctx.permissions.allowed_for_tool(id))
        .map(|id| stock_tool_schema(id))
        .collect();

    // Sort by name for deterministic output
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    schemas
}

/// Return the schema for a stock tool by ID.
///
/// In the full system this would come from a `ToolRegistry`; for now we
/// provide schemas for the Sprint 4 stock tools.
#[must_use]
#[allow(clippy::too_many_lines)]
fn stock_tool_schema(tool_id: &str) -> ToolSchema {
    match tool_id {
        "fs_read" => ToolSchema {
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
                "required": ["path"]
            }),
        },
        "fs_write" => ToolSchema {
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
                "required": ["path", "content"]
            }),
        },
        "fs_edit" => ToolSchema {
            name: "fs_edit".to_owned(),
            description: "Edit a file by replacing an exact string match. The old_string \
                          must appear exactly once in the file."
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
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        },
        "grep" => ToolSchema {
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
                "required": ["pattern"]
            }),
        },
        "glob" => ToolSchema {
            name: "glob".to_owned(),
            description: "Find files matching a glob pattern. Returns a list of \
                          matching file paths."
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
                "required": ["pattern"]
            }),
        },
        "bash" => ToolSchema {
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
                "required": ["command"]
            }),
        },
        "sub_agent" => ToolSchema {
            name: "sub_agent".to_owned(),
            description: "Spawn a child agent with a bounded budget for a sub-task. \
                          The child agent has its own reasoning trail."
                .to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "Task description for the child agent"
                    },
                    "max_tokens": {
                        "type": "integer",
                        "description": "Token budget for the child agent"
                    }
                },
                "required": ["prompt"]
            }),
        },
        // Unknown tool — return a minimal schema
        other => ToolSchema {
            name: other.to_owned(),
            description: format!("Tool: {other}"),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turn_context::{
        HostOs, PermissionView, TabSnapshot, UserMessage,
    };

    #[test]
    fn filters_by_permission() {
        let ctx = TurnContext {
            turn_id: "t1".into(),
            user_turn: UserMessage { text: "hi".into() },
            history: Default::default(),
            model_profile: rustacle_llm::ModelProfile {
                name: "test".into(),
                provider: "local".into(),
                model: "test".into(),
                api_base: None,
                max_tokens: None,
                temperature: None,
            },
            ui_enabled_tools: vec!["fs_read".into(), "bash".into(), "grep".into()],
            active_tab: TabSnapshot {
                index: 0,
                title: "main".into(),
                cwd: "/tmp".into(),
                shell_name: "bash".into(),
                shell_path: "/bin/bash".into(),
                last_commands: vec![],
            },
            open_tabs: vec![],
            host_os: HostOs {
                name: "Linux".into(),
                version: "6.8.0".into(),
            },
            permissions: PermissionView {
                granted_tools: vec!["fs_read".into(), "grep".into()],
            },
            project_docs: Default::default(),
            memory: Default::default(),
            selected_files: vec![],
            now: 0,
            timezone: "UTC".into(),
            extra: Default::default(),
        };

        let schemas = collect_tool_schemas(&ctx);
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        // bash is UI-enabled but NOT permission-granted → filtered out
        assert_eq!(names, vec!["fs_read", "grep"]);
    }

    #[test]
    fn schemas_sorted_by_name() {
        let ctx = TurnContext {
            turn_id: "t1".into(),
            user_turn: UserMessage { text: "hi".into() },
            history: Default::default(),
            model_profile: rustacle_llm::ModelProfile {
                name: "test".into(),
                provider: "local".into(),
                model: "test".into(),
                api_base: None,
                max_tokens: None,
                temperature: None,
            },
            ui_enabled_tools: vec!["grep".into(), "bash".into(), "fs_read".into()],
            active_tab: TabSnapshot {
                index: 0,
                title: "main".into(),
                cwd: "/tmp".into(),
                shell_name: "bash".into(),
                shell_path: "/bin/bash".into(),
                last_commands: vec![],
            },
            open_tabs: vec![],
            host_os: HostOs {
                name: "Linux".into(),
                version: "6.8.0".into(),
            },
            permissions: PermissionView {
                granted_tools: vec!["grep".into(), "bash".into(), "fs_read".into()],
            },
            project_docs: Default::default(),
            memory: Default::default(),
            selected_files: vec![],
            now: 0,
            timezone: "UTC".into(),
            extra: Default::default(),
        };

        let schemas = collect_tool_schemas(&ctx);
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["bash", "fs_read", "grep"]);
    }
}
