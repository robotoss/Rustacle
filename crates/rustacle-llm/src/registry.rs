use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::provider::{ChatStream, LlmError, LlmProvider};
use crate::types::{ChatRequest, ModelProfile};

/// Routes LLM requests to the appropriate provider based on model profiles.
pub struct LlmRegistry {
    providers: RwLock<HashMap<String, Arc<dyn LlmProvider>>>,
    profiles: RwLock<HashMap<String, ModelProfile>>,
}

impl LlmRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            profiles: RwLock::new(HashMap::new()),
        }
    }

    /// Register a provider by its ID.
    pub async fn register_provider(&self, provider: Arc<dyn LlmProvider>) {
        let id = provider.id().to_string();
        tracing::info!(provider.id = %id, provider.name = provider.name(), "LLM provider registered");
        self.providers.write().await.insert(id, provider);
    }

    /// Add a model profile.
    pub async fn add_profile(&self, profile: ModelProfile) {
        tracing::info!(profile.name = %profile.name, provider = %profile.provider, model = %profile.model, "model profile added");
        self.profiles
            .write()
            .await
            .insert(profile.name.clone(), profile);
    }

    /// Stream a chat completion using a named profile.
    ///
    /// # Errors
    /// Returns `LlmError::Config` if the profile or provider is not found.
    pub async fn stream(
        &self,
        profile_name: &str,
        mut request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<ChatStream, LlmError> {
        let profiles = self.profiles.read().await;
        let profile = profiles
            .get(profile_name)
            .ok_or_else(|| LlmError::Config(format!("profile not found: {profile_name}")))?
            .clone();
        drop(profiles);

        // Override request fields from profile.
        request.model = profile.model.clone();
        if let Some(max) = profile.max_tokens {
            request.max_tokens = Some(max);
        }
        if let Some(temp) = profile.temperature {
            request.temperature = Some(temp);
        }

        let providers = self.providers.read().await;
        let provider = providers
            .get(&profile.provider)
            .ok_or_else(|| LlmError::Config(format!("provider not found: {}", profile.provider)))?
            .clone();
        drop(providers);

        provider.stream(request, cancel).await
    }

    /// List all registered profile names.
    pub async fn list_profiles(&self) -> Vec<String> {
        self.profiles.read().await.keys().cloned().collect()
    }

    /// List all registered provider IDs.
    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }
}

impl Default for LlmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_profile_not_found() {
        let registry = LlmRegistry::new();
        let request = ChatRequest {
            model: String::new(),
            messages: vec![],
            tools: vec![],
            max_tokens: None,
            temperature: None,
        };
        let result = registry
            .stream("nonexistent", request, CancellationToken::new())
            .await;
        assert!(result.is_err());
    }
}
