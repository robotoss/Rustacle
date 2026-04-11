pub mod streaming;

use rustacle_llm::provider::{ChatStream, LlmError, LlmProvider};
use rustacle_llm::types::ChatRequest;
use tokio_util::sync::CancellationToken;

/// OpenAI-compatible LLM provider. Also works with Ollama, LM Studio, vLLM.
pub struct OpenAiProvider {
    api_base: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Create a new provider targeting the given API base URL.
    #[must_use]
    pub fn new(api_base: String, api_key: Option<String>) -> Self {
        Self {
            api_base,
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)] // trait returns &str, impls return literals
impl LlmProvider for OpenAiProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn name(&self) -> &str {
        "OpenAI Compatible"
    }

    async fn stream(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<ChatStream, LlmError> {
        streaming::stream_openai(&self.client, &self.api_base, self.api_key.as_deref(), request, cancel).await
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/models", self.api_base);
        let mut req = self.client.get(&url);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req.send().await.map_err(|e| LlmError::Provider {
            provider: "openai".to_string(),
            message: e.to_string(),
            retryable: true,
        })?;

        let body: serde_json::Value = resp.json().await.map_err(|e| LlmError::Provider {
            provider: "openai".to_string(),
            message: e.to_string(),
            retryable: false,
        })?;

        let models = body["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}
