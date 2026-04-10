# Rustacle — Architectural Foundation

> A next-generation **Agentic Terminal** built on Rust + Tauri. Micro-kernel at the core, hot-pluggable WASM modules at the edges, visible agent reasoning in the middle, and **zero** hand-edited JSON in the user's face.

This directory is the project's **architectural canon**. No code lives here — only the blueprints an engineer needs to open a pull request with confidence. If a decision isn't written down in `for_dev/`, it isn't a decision yet.

---

## Start Here

- **[`concept.md`](./concept.md)** — the one-page answer to "what is this and why?". Read first.
- **[`glossary.md`](./glossary.md)** — every load-bearing term defined once. Keep it open while reading other docs.

## Document Map

### Vision & principles
| Document | What it answers |
|---|---|
| [`concept.md`](./concept.md) | What is Rustacle? Why does it exist? How does it differ from existing tools? |
| [`ui_simplicity.md`](./ui_simplicity.md) | What "simple" means here; how we stand apart from Claude Code CLI and IDE chat panels. |
| [`glossary.md`](./glossary.md) | Definitions of every term (turn, step, capability, profile, …). |

### How it's built
| Document | What it answers |
|---|---|
| [`architecture.md`](./architecture.md) | Micro-kernel, IPC, WIT contract, plugin trait, event bus, permission broker. |
| [`project_structure.md`](./project_structure.md) | Full Cargo workspace tree, every crate, every src file. |
| [`modularity.md`](./modularity.md) | Core vs plugins, extension points, event-bus-as-nervous-system, many-agent patterns. |
| [`tech_stack_2026.md`](./tech_stack_2026.md) | Every crate and why, with alternatives rejected. |

### How the agent thinks
| Document | What it answers |
|---|---|
| [`agent_reasoning.md`](./agent_reasoning.md) | Harness loop, prompt assembly, tool dispatch, LlmProvider trait. |
| [`prompts_catalog.md`](./prompts_catalog.md) | Every system-level prompt verbatim: SYSTEM_BASE, overlays, env template, reminders. |
| [`tools_catalog.md`](./tools_catalog.md) | Every stock tool: description, JSON Schema, concurrency, capabilities. |

### Integrations & platform reality
| Document | What it answers |
|---|---|
| [`mcp_and_models.md`](./mcp_and_models.md) | Providers (OpenAI/Anthropic/Ollama/LM Studio/…), MCP servers, configuration UX. |
| [`cross_platform.md`](./cross_platform.md) | Windows/macOS/Linux specifics: PTY, keyring, paths, signals, packaging. |

### Security, visibility, UX
| Document | What it answers |
|---|---|
| [`security.md`](./security.md) | Sandbox layers, command execution isolation, agent protection, protection from agent, audit trail. |
| [`observability.md`](./observability.md) | Logging, tracing, trace IDs, token/cost accounting, replay, metrics, OTLP. |
| [`ui_ux_manifesto.md`](./ui_ux_manifesto.md) | Zero-JSON rule, multi-window terminal, visible agent panel, themes, a11y, first-run flow. |

### Delivery
| Document | What it answers |
|---|---|
| [`roadmap.md`](./roadmap.md) | Sprint 0 → Sprint 8 plan with exit criteria and risks. |
| [`knowledge_base.md`](./knowledge_base.md) | Rust memory patterns, TS↔Rust error hygiene, DX checklist, threat tables. |

### Decisions
| Document | What it answers |
|---|---|
| [`adr/0001-ui-framework.md`](./adr/0001-ui-framework.md) | Solid vs React. |

### Reading order for a new engineer
`concept → glossary → ui_simplicity → architecture → project_structure → modularity → agent_reasoning → tools_catalog → prompts_catalog → mcp_and_models → ui_ux_manifesto → security → observability → cross_platform → tech_stack_2026 → knowledge_base → roadmap → adr/*`.

---

## Inspirations (read-only references)

| Source | What we borrow | Where it's cited |
|---|---|---|
| `refs/cc-src/query.ts`, `QueryEngine.ts`, `Tool.ts`, `tools/*`, `constants/prompts.ts` | Harness loop, tool dispatch, streaming events, `buildTool` factory, dynamic system-prompt assembly. | `agent_reasoning.md`, `tools_catalog.md`, `prompts_catalog.md` |
| [`claude-code-from-scratch` (FareedKhan-dev)](https://github.com/FareedKhan-dev/claude-code-from-scratch) | Event-bus-centric design, heavy modularity, multi-agent patterns. | `modularity.md`, `concept.md` |
| `refs/acc` | Modular crate discipline, minimal Tauri shell. | `architecture.md`, `project_structure.md` |
| `refs/claw-code` (`PHILOSOPHY.md`, `BRAIN.md`, `PARITY.md`) | "Humans direct, agents execute"; parity/sprint checkpoint discipline. | `concept.md`, `roadmap.md` |
| [opencode](https://github.com/anomalyco/opencode) | Agentic patterns; local-first provider posture. | `agent_reasoning.md`, `mcp_and_models.md` |
| [Harness Engineering (article)](https://levelup.gitconnected.com/building-claude-code-with-harness-engineering-d2e8c0da85f0) | The "harness" mental model: one event loop, cancel-safe futures, dispatch tables. | `agent_reasoning.md` §5 |
| [Model Context Protocol](https://modelcontextprotocol.io) | External tool / resource / prompt integration. | `mcp_and_models.md` |

---

## Non-Negotiables (hard rules)

1. **Micro-kernel, not monolith.** The kernel does lifecycle, IPC routing, permissions, event bus — and nothing else. Every feature ships as a plugin.
2. **Plugins are WASM-first.** Native plugins exist **only** where WASI cannot yet reach (PTY spawning today). Each native plugin carries an explicit migration note.
3. **Reasoning is visible.** Every `Thought`, `ToolCall`, `ToolResult`, `PermissionAsk`, `Answer` streams to the UI as a first-class typed event. No hidden reasoning.
4. **Zero-JSON UX.** End users never open a config file. Every setting ships with a UI control in the same PR.
5. **Capability-gated everything.** No plugin performs I/O without going through the Permission Broker.
6. **Type-safe bridge.** `specta + tauri-specta` generate `bindings.ts` from Rust. A hand-written TS IPC type is a bug.
7. **Deterministic prompts.** Given identical inputs, `assemble_prompt()` must produce byte-identical output.
8. **Cancel-safe futures.** Every `.await` inside a turn must be cancel-safe. Orphaned `tokio::spawn` is a bug.
9. **Local-first.** Any feature that works with a cloud model must work with a local one.
10. **Signed everywhere.** Every plugin and every update is signed.

Violating any of these without an accompanying ADR is grounds for PR rejection.

---

## Status Legend

| Status | Meaning |
|---|---|
| 🟢 Stable | Ratified; changes require an ADR. |
| 🟡 Draft | Under active iteration; may change until Sprint 0 ends. |
| 🔴 Placeholder | Not yet written to depth. |

Current state: **all docs are 🟡 Draft** until Sprint 0 ends with ADR-0002 "freeze foundation".

---
*All documents link back here. See the map above.*
