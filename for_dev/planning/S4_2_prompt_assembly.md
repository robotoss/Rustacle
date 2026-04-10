# S4.2 — Deterministic Prompt Assembly

## Goal
Implement the deterministic 8-layer `assemble_prompt()` function with golden tests via `insta` snapshots, ensuring byte-identical output for identical input.

## Context
Prompt assembly is the most architecturally sensitive code. Given identical `TurnContext`, two calls must produce byte-identical prompts. The 8 layers are fixed in order: system_base, model_profile, env_context, tools, project_docs, selected_files, memory, history + user turn. Each layer is independently rendered and budget-aware. This is the foundation for all agent behavior.

## Docs to read
- `for_dev/agent_reasoning.md` section 3 — Prompt Assembly: all 10 principles, full pseudocode, `TurnContext` definition.
- `for_dev/agent_reasoning.md` section 3.1 — `TurnContext` struct fields.
- `for_dev/agent_reasoning.md` section 3.3 — complete `assemble_prompt()` pseudocode.
- `for_dev/prompts_catalog.md` — all prompt fragments verbatim (SYSTEM_BASE, tool instructions, etc.).
- `for_dev/project_structure.md` section `plugins/agent/src/prompt/` — expected module layout.

## Reference code
- `refs/cc-src/constants/prompts.ts::getSystemPrompt` (line 444) — composition pattern for layered prompt building.
- `for_dev/agent_reasoning.md` section 3.3 — complete pseudocode to translate into Rust.

## Deliverables
```
plugins/agent/src/prompt/
├── mod.rs              # assemble_prompt(ctx: &TurnContext) -> Vec<ChatMessage>
├── layers.rs           # SYSTEM_BASE constant, render_env_context(), render functions per layer
├── tools.rs            # render_tool_schemas() for provider dialect (OpenAI, Anthropic)
└── golden_tests.rs     # insta snapshot tests: fixture_a (baseline), cwd_change_test

plugins/agent/src/
└── turn_context.rs     # TurnContext struct with all fields from agent_reasoning.md §3.1
```

## Checklist
- [ ] `TurnContext` struct has all fields from agent_reasoning.md section 3.1
- [ ] `assemble_prompt()` is deterministic: no `HashMap` iteration, no wall-clock, no random values
- [ ] All 8 layers present in fixed order: system_base → model_profile → env_context → tools → project_docs → selected_files → memory → history + user turn
- [ ] `SYSTEM_BASE` constant matches `prompts_catalog.md` section 1 verbatim
- [ ] Tools are filtered by UI-enabled flag and permission grants before rendering
- [ ] Tool schemas rendered in provider's native tool-use dialect (not prose)
- [ ] Project docs are walked up from cwd to repo root
- [ ] Memory entries are scored against the user turn only (relevance ranking)
- [ ] History is trimmed: first user message + last N messages preserved
- [ ] Budget-aware truncation applied per layer (respects token limits)
- [ ] At least 2 golden test snapshots committed via `insta`
- [ ] Changing cwd in `TurnContext` changes only the `env_context` layer in output
- [ ] `BTreeMap` or sorted `Vec` used wherever ordering matters

## Acceptance criteria
```bash
# Agent plugin compiles
cargo check -p rustacle-agent

# Golden tests pass
cargo test -p rustacle-agent -- prompt

# Verify snapshots exist
ls plugins/agent/src/prompt/snapshots/

# Insta review (no pending changes)
cargo insta test -p rustacle-agent --review

# Workspace compiles
cargo check --workspace
```

## Anti-patterns
- Do NOT use `HashMap` for any data that affects prompt output — use `BTreeMap` or sorted `Vec`.
- Do NOT let layers inspect or mutate each other — each layer renders independently from `TurnContext`.
- Do NOT put tool schemas in prose format — use the provider's structured tool-use dialect.
- Do NOT leak secrets (API keys, tokens) into prompts.
- Do NOT use wall-clock time or random values in assembly — determinism is a hard requirement.
- Do NOT skip budget tracking — each layer must respect its allocated token budget.
