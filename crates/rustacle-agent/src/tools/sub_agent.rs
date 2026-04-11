//! `sub_agent` tool: spawn a child harness with bounded budget.

use async_trait::async_trait;
use serde_json::Value;

use rustacle_llm::types::ToolSchema;

use super::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};

/// Spawn a child agent with its own bounded budget.
pub struct SubAgentTool;

#[async_trait]
impl Tool for SubAgentTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "sub_agent".to_owned(),
            description: "Spawn a child agent with a bounded budget for a sub-task. \
                          The child agent has its own reasoning trail visible as \
                          a nested subtree in the UI."
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
                        "description": "Token budget for the child agent (default: 10000)"
                    }
                },
                "required": ["prompt"],
                "additionalProperties": false
            }),
        }
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        args.get("prompt")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .ok_or_else(|| ToolError::Validation("prompt is required".into()))?;
        Ok(())
    }

    fn concurrency(&self) -> Concurrency {
        Concurrency::Serialized
    }

    fn required_capabilities(&self, _args: &Value) -> Vec<Capability> {
        vec![Capability::LlmProvider]
    }

    async fn call(&self, args: Value, _ctx: ToolCtx) -> Result<ToolOutput, ToolError> {
        let prompt = args["prompt"].as_str().unwrap_or("");
        let _max_tokens = args
            .get("max_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(10_000);

        // In the full system, this spawns a child Harness with:
        // - parent_id set to the current step
        // - bounded budget (max_tokens, max_tool_calls)
        // - its own cancel token (child of parent's)
        //
        // For now, return a placeholder indicating what would happen.
        let summary = format!(
            "[sub_agent] would spawn child harness for: {}",
            if prompt.len() > 100 {
                &prompt[..100]
            } else {
                prompt
            }
        );

        Ok(ToolOutput {
            summary,
            payload: None,
        })
    }
}
