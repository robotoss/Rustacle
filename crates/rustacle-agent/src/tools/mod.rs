//! Tool dispatch table and stock tools.
//!
//! Each tool implements the [`Tool`] trait. The [`ToolDispatchTable`] partitions
//! calls into concurrent and serialized sets for fan-out execution.

mod registry;

pub mod bash;
pub mod fs_edit;
pub mod fs_read;
pub mod fs_write;
pub mod glob;
pub mod grep;
pub mod sub_agent;

pub use registry::ToolDispatchTable;

use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use rustacle_llm::types::ToolSchema;

/// Tool concurrency mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Concurrency {
    /// Safe to run in parallel with other concurrent tools.
    Concurrent,
    /// Must run alone; dispatcher drains all in-flight first.
    Serialized,
}

/// A capability required by a tool call.
#[derive(Debug, Clone)]
pub enum Capability {
    /// Filesystem access.
    Fs { path: String, write: bool },
    /// PTY / shell access.
    Pty,
    /// LLM provider access (for sub-agent).
    LlmProvider,
}

/// Output from a tool execution.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub summary: String,
    pub payload: Option<bytes::Bytes>,
}

/// Errors from tool execution.
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    #[error("validation: {0}")]
    Validation(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("execution: {0}")]
    Execution(String),

    #[error("cancelled")]
    Cancelled,
}

/// Context passed to a tool during execution.
pub struct ToolCtx {
    pub cancel: CancellationToken,
    pub cwd: std::path::PathBuf,
}

/// The tool trait. Every stock and user-defined tool implements this.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool schema for the LLM's tool-use dialect.
    fn schema(&self) -> ToolSchema;

    /// Tool-level prompt contribution appended to the tool description.
    fn prompt_addendum(&self) -> &'static str {
        ""
    }

    /// Cheap synchronous validation. Runs before permission check.
    ///
    /// # Errors
    /// Returns `ToolError::Validation` if args are invalid.
    fn validate(&self, args: &Value) -> Result<(), ToolError>;

    /// Concurrency mode.
    fn concurrency(&self) -> Concurrency;

    /// Capabilities required for this specific call.
    fn required_capabilities(&self, args: &Value) -> Vec<Capability>;

    /// Execute the tool.
    async fn call(&self, args: Value, ctx: ToolCtx) -> Result<ToolOutput, ToolError>;
}

/// Placeholder for when no real tools are registered yet.
/// Returns a description of what would happen.
pub(crate) fn placeholder_dispatch(tool_name: &str, args: &Value) -> String {
    format!(
        "[placeholder] tool={tool_name} args={}",
        serde_json::to_string(args).unwrap_or_default()
    )
}
