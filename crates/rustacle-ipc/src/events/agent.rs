use serde::{Deserialize, Serialize};

/// A step in the agent's reasoning process, streamed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ReasoningStepEvent {
    pub id: String,
    pub parent_id: Option<String>,
    pub turn_id: String,
    pub ts_ms: u64,
    pub step: ReasoningStep,
}

/// The type of reasoning step.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "data")]
pub enum ReasoningStep {
    Thought {
        text: String,
        partial: bool,
    },
    ToolCall {
        tool: String,
        args: serde_json::Value,
        tab_target: Option<u32>,
    },
    ToolResult {
        tool: String,
        ok: bool,
        summary: String,
        duration_ms: u32,
    },
    PermissionAsk {
        capability: String,
        decision: Option<String>,
    },
    Answer {
        text: String,
    },
    Error {
        message: String,
        retryable: bool,
    },
}

/// Cost sample streamed periodically during a turn.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CostSampleEvent {
    pub turn_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Emitted when a turn finishes.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TurnEndEvent {
    pub turn_id: String,
    pub duration_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tool_calls: u32,
}
