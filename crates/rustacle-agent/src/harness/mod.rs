//! The agent harness: ReAct-style thinking loop.
//!
//! Entry point is [`Harness::run_turn`] which drives the cycle:
//! assemble prompt -> stream LLM -> parse deltas -> dispatch tools -> repeat.

mod cancel;
mod r#loop;
mod streaming;

pub use cancel::CancelHandle;
pub use r#loop::Harness;
pub use streaming::FlushConfig;

use serde::{Deserialize, Serialize};

/// Unique step identifier (ULID).
pub type StepId = String;

/// Unique turn identifier.
pub type TurnId = String;

/// A reasoning step emitted during a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub id: StepId,
    pub parent_id: Option<StepId>,
    pub turn_id: TurnId,
    pub ts_ms: u64,
    pub kind: StepKind,
}

/// The type of reasoning step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum StepKind {
    /// Partial or complete thought text from the LLM.
    Thought { text: String, partial: bool },
    /// The agent is calling a tool.
    ToolCall {
        tool: String,
        args: serde_json::Value,
        tab_target: Option<usize>,
    },
    /// Result from a tool execution.
    ToolResult {
        tool: String,
        ok: bool,
        summary: String,
        duration_ms: u32,
    },
    /// Permission request for a capability.
    PermissionAsk {
        capability: String,
        decision: Option<PermissionDecision>,
    },
    /// Final answer from the agent.
    Answer { text: String },
    /// Error during the turn.
    Error { message: String, retryable: bool },
}

/// Permission decision from the user.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PermissionDecision {
    Deny,
    AllowOnce,
    AllowAlways,
}

/// Budget limits for a turn.
#[derive(Debug, Clone)]
pub struct TurnBudget {
    /// Maximum number of tool calls per turn.
    pub max_tool_calls: u32,
    /// Maximum duration in milliseconds.
    pub max_duration_ms: u64,
    /// Maximum total tokens (input + output).
    pub max_tokens: u64,
}

impl Default for TurnBudget {
    fn default() -> Self {
        Self {
            max_tool_calls: 50,
            max_duration_ms: 300_000, // 5 minutes
            max_tokens: 100_000,
        }
    }
}

/// Cost tracking for a turn.
#[derive(Debug, Clone, Default)]
pub struct TurnCost {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tool_calls: u32,
    pub elapsed_ms: u64,
}

impl TurnCost {
    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Check if any budget limit is exceeded.
    #[must_use]
    pub fn exceeds(&self, budget: &TurnBudget) -> Option<&'static str> {
        if self.tool_calls >= budget.max_tool_calls {
            Some("max tool calls exceeded")
        } else if self.elapsed_ms >= budget.max_duration_ms {
            Some("max duration exceeded")
        } else if self.total_tokens() >= budget.max_tokens {
            Some("max tokens exceeded")
        } else {
            None
        }
    }
}

/// Errors from the harness.
#[derive(thiserror::Error, Debug)]
pub enum HarnessError {
    #[error("cancelled")]
    Cancelled,

    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("llm error: {0}")]
    Llm(#[from] rustacle_llm::provider::LlmError),

    #[error("tool error: {tool}: {message}")]
    Tool { tool: String, message: String },

    #[error("internal: {0}")]
    Internal(String),
}
