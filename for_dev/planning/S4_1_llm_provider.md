# S4.1 — LLM Provider Trait & OpenAI Implementation

## Goal
Create `rustacle-llm` with the `LlmProvider` trait and `LlmRegistry`, plus the first provider implementations in `rustacle-llm-openai` (remote APIs) and `rustacle-llm-local` (localhost auto-discovery).

## Context
The LLM layer lives on the host, not inside plugins. Plugins call host functions (`llm-stream`, `llm-poll`); the host routes to the configured provider via the registry. The OpenAI provider also works for Ollama, LM Studio, and vLLM (OpenAI-compatible APIs). Local discovery probes well-known localhost ports to find running inference servers.

## Docs to read
- `for_dev/agent_reasoning.md` section 6 — Multi-Model Support: full `LlmProvider` trait code, `ModelProfile`, `ProviderCapabilities`.
- `for_dev/agent_reasoning.md` sections 6.1–6.3 — complete Rust source for trait, registry, and routing.
- `for_dev/architecture.md` section 4.2 — host interface `llm-stream` / `llm-poll` functions.
- `for_dev/tech_stack_2026.md` section 5 — LLM and Streaming stack (async-openai, eventsource-stream).
- `for_dev/project_structure.md` sections `rustacle-llm*` — crate layout.

## Reference code
- `for_dev/agent_reasoning.md` sections 6.1–6.3 — copy trait and type definitions verbatim.
- Internet: [`async-openai` crate](https://docs.rs/async-openai), [`eventsource-stream`](https://docs.rs/eventsource-stream), [OpenAI streaming API](https://platform.openai.com/docs/api-reference/streaming), [Ollama API compatibility](https://github.com/ollama/ollama/blob/main/docs/openai.md).

## Deliverables
```
crates/rustacle-llm/src/
├── lib.rs              # Re-exports
├── provider.rs         # LlmProvider async trait, ProviderCapabilities
├── registry.rs         # LlmRegistry: register, get by ModelProfile
├── types.rs            # ChatRequest, ChatDelta enum, ToolSchema, TokenCost
└── router.rs           # Bridges plugin llm-stream host fn → provider

crates/rustacle-llm-openai/src/
├── lib.rs              # impl LlmProvider for OpenAiProvider
└── streaming.rs        # SSE parsing, tool-use delta translation

crates/rustacle-llm-local/src/
├── lib.rs              # impl LlmProvider for LocalProvider
└── discovery.rs        # Auto-detect localhost servers (port probing)
```

## Checklist
- [ ] `LlmProvider` trait compiles with async stream return type (`Stream<Item = ChatDelta>`)
- [ ] `ChatDelta` enum has variants: `Text`, `ToolUseStart`, `ToolUseDelta`, `ToolUseEnd`, `Usage`, `Done`
- [ ] `OpenAiProvider` streams completions from a configurable API endpoint
- [ ] SSE parsing handles partial chunks and reconnection
- [ ] Tool-use deltas are translated from OpenAI's JSON format to `ChatDelta` variants
- [ ] `LocalProvider` discovery probes standard ports (11434 for Ollama, 1234 for LM Studio)
- [ ] `LlmRegistry` routes requests by `ModelProfile` to the correct provider
- [ ] Cancel token (`CancellationToken`) stops in-flight streams
- [ ] `router.rs` bridges `llm-stream` host function calls to the registry
- [ ] `TokenCost` tracks prompt/completion token counts per request

## Acceptance criteria
```bash
# All LLM crates compile
cargo check -p rustacle-llm -p rustacle-llm-openai -p rustacle-llm-local

# Unit tests pass
cargo test -p rustacle-llm
cargo test -p rustacle-llm-openai
cargo test -p rustacle-llm-local

# Workspace compiles
cargo check --workspace
```

## Anti-patterns
- Do NOT let plugins talk HTTP directly to LLM APIs — they must go through host functions.
- Do NOT hardcode API URLs — they come from `ModelProfile` configuration.
- Do NOT skip cancel support on streams — every stream must respect `CancellationToken`.
- Do NOT block on stream polling — use proper async iteration.
- Do NOT store API keys in code — they come from the credential store at runtime.
