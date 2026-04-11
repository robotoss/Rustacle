use std::pin::Pin;

use futures_core::Stream;
use tokio_util::sync::CancellationToken;

use crate::types::{ChatDelta, ChatRequest};

/// Errors from an LLM provider.
#[derive(thiserror::Error, Debug)]
pub enum LlmError {
    #[error("provider error: {provider}: {message}")]
    Provider {
        provider: String,
        message: String,
        retryable: bool,
    },

    #[error("cancelled")]
    Cancelled,

    #[error("configuration error: {0}")]
    Config(String),
}

/// A boxed async stream of chat deltas.
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatDelta, LlmError>> + Send>>;

/// Trait for LLM providers (OpenAI-compatible, Anthropic, local servers).
///
/// Providers live on the host, not in plugins. Plugins call `llm-stream`
/// host functions; the host routes to the configured provider.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider identifier (e.g., "openai", "anthropic", "local").
    fn id(&self) -> &str;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Start a streaming chat completion.
    ///
    /// # Errors
    /// Returns `LlmError` on network, auth, or model errors.
    async fn stream(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<ChatStream, LlmError>;

    /// List available models from this provider.
    ///
    /// # Errors
    /// Returns `LlmError` if the provider cannot be reached.
    async fn list_models(&self) -> Result<Vec<String>, LlmError>;
}
