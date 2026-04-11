pub mod discovery;

use rustacle_llm::provider::{ChatStream, LlmError, LlmProvider};
use rustacle_llm::types::ChatRequest;
use rustacle_llm_openai::OpenAiProvider;
use tokio_util::sync::CancellationToken;

/// Local LLM provider — wraps an `OpenAiProvider` pointing at a local server.
///
/// Ollama, LM Studio, llama.cpp-server, and vLLM all expose OpenAI-compatible APIs.
pub struct LocalProvider {
    inner: OpenAiProvider,
    server_name: String,
}

impl LocalProvider {
    /// Create a local provider from a discovered server.
    #[must_use]
    pub fn new(server_name: String, api_base: String) -> Self {
        Self {
            inner: OpenAiProvider::new(api_base, None),
            server_name,
        }
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl LlmProvider for LocalProvider {
    fn id(&self) -> &str {
        "local"
    }

    fn name(&self) -> &str {
        &self.server_name
    }

    async fn stream(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<ChatStream, LlmError> {
        self.inner.stream(request, cancel).await
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.list_models().await
    }
}
