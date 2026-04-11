use serde::{Deserialize, Serialize};

/// A step in the agent's reasoning process, streamed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ReasoningStepEvent {
    pub turn_id: String,
    pub step: ReasoningStep,
}

/// The type of reasoning step.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "data")]
pub enum ReasoningStep {
    Thought { text: String },
    ToolCall { tool: String, input: String },
    ToolResult { tool: String, output: String },
    Answer { text: String },
}

/// Cost sample streamed periodically during a turn.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct CostSampleEvent {
    pub turn_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}
