/// LLM bridge stub.
///
/// Provides `llm-stream` and `llm-poll` host functions to plugins.
/// Real implementation arrives in Sprint 4 (LLM provider trait).
pub struct LlmBridge;

impl LlmBridge {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for LlmBridge {
    fn default() -> Self {
        Self::new()
    }
}
