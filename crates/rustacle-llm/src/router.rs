/// LLM router bridge.
///
/// Bridges `llm-stream` / `llm-poll` host function calls from WASM plugins
/// to the `LlmRegistry`. Full implementation arrives when the WASM host
/// linker is wired up (S2.2 linker.rs).
///
/// For now, the agent plugin calls the registry directly as a native plugin.
pub struct LlmRouter;

impl LlmRouter {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for LlmRouter {
    fn default() -> Self {
        Self::new()
    }
}
