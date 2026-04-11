# Rustacle — Concept & Vision

> The one-page answer to "what is this and why does it exist?" Read before anything else.

---

## 1. The one-line pitch

**Rustacle is a local-first desktop agent controller where the agent's reasoning is visible, every action is permissioned, every setting lives in a UI, and every feature is a hot-swappable module.**

## 2. The problem

Today's "AI terminals" and "AI code assistants" split into two camps:

- **Opaque cloud CLIs** (e.g. classic Claude Code CLI). Powerful, but the reasoning is hidden behind a chat view, configuration is scattered across JSON/YAML files, and extension is limited to hand-edited "skills" or shell hacks.
- **IDE chat panels**. Focused on editing, weak at orchestrating real terminal work across multiple shells, weaker still at running as a standalone daily driver.

Neither gives the power user what they actually want:

1. **Visibility** — see what the agent is thinking, see every tool call before it happens, stop it cleanly.
2. **Control** — decide which tools exist, which models run, which files the agent can touch, without editing a config file.
3. **Locality** — run local models the same way as cloud ones, keep data on disk, work offline.
4. **Extensibility** — add a tool, replace a provider, ship a team-wide module, without recompiling the host.
5. **Trust** — know the agent cannot escape its sandbox, cannot see secrets it wasn't granted, cannot run a command without the user authorizing the capability once.

## 3. The product

A native desktop agent controller on Rust + Tauri with:

- **Multi-tab, multi-split shells** with per-tab agent context and tool-use redirection (see [`ui_ux_manifesto.md` §2](./ui_ux_manifesto.md)).
- **A visible Agent Panel** streaming the agent's `Thought → ToolCall → Observation → Answer` as typed event cards in real time (see [`agent_reasoning.md` §1](./agent_reasoning.md)).
- **A Zero-JSON Settings UI** — every setting typed and UI-editable, never a config file (see [`ui_ux_manifesto.md` §1](./ui_ux_manifesto.md)).
- **Local and cloud models** with one-click provider setup, auto-discovery of Ollama / LM Studio / llama.cpp-server (see [`mcp_and_models.md`](./mcp_and_models.md)).
- **MCP support** for external tools and resources, gated by the same permission broker as built-in tools.
- **A WASM plugin sandbox** — every feature (chat, FS, memory, skills, even the agent itself) is a hot-swappable, capability-gated, signed module.
- **A small, audited micro-kernel** that does only lifecycle, IPC, permissions, and the event bus.

## 4. What makes it different

| | Rustacle | Typical CLI AI tool | IDE chat panel |
|---|---|---|---|
| Visible reasoning | ✅ first-class typed events | ⚠️ prose, sometimes hidden | ⚠️ in a sidebar |
| Zero-JSON config | ✅ hard rule | ❌ YAML / JSON everywhere | mixed |
| Per-tool permissions UI | ✅ typed capability broker | ⚠️ allow-lists in files | ⚠️ |
| Hot-swap plugins | ✅ signed WASM | ❌ | ❌ |
| Multi-tab shell context | ✅ native | ❌ | ❌ |
| Local + cloud providers | ✅ one trait, UI selects | mixed | mixed |
| MCP servers as peers | ✅ through permission broker | recent | ⚠️ |
| Deterministic prompts | ✅ golden-tested | ❌ | ❌ |
| Cross-platform native | ✅ Win/macOS/Linux, Tauri v2 | mixed | depends on IDE |
| Cancel-safe turns | ✅ one token per turn | ⚠️ | ⚠️ |
| Visible token / cost tracking | ✅ live badge, per-tool breakdown | ⚠️ hidden | ⚠️ |
| Sandbox posture | ✅ fuel-metered WASM + capability broker | ❌ | ❌ |

The thesis: **transparency + control + modularity** are not features you add later. They must be in the foundation, or they never show up.

## 5. Principles

The rules we refuse to break. If a design choice conflicts with a principle, the design changes.

1. **Visibility over cleverness.** The user always sees what the agent is doing. Hidden reasoning is a bug.
2. **Capability over blanket trust.** Nothing the agent touches bypasses the permission broker.
3. **UI over config files.** Users never open a JSON file to change behavior.
4. **Determinism over spontaneity.** Given the same inputs, the same prompt, the same result, the same golden test.
5. **Modules over monolith.** If it can be a plugin, it is a plugin.
6. **Local-first over cloud-only.** Any feature that works with a cloud model must work with a local one. No feature is gated behind a cloud-only provider.
7. **Types over strings.** IPC, events, errors, settings — all typed, all cross-compiled to TS.
8. **Cancellation over retry.** Users should be able to stop anything in under 100 ms.
9. **Signed over trusted.** Every plugin and every update is signed.
10. **Small kernel, rich edges.** The kernel stays boring; features live in plugins.

## 6. Non-goals

Things we explicitly decided not to do in 1.0, to keep scope sane.

- **Not an IDE.** No syntax-aware editor, no language servers. Open files in a preview pane at most.
- **Not a cloud service.** No server-side storage, no "team account", no login. Everything local.
- **Not a fine-tuning platform.** We call models; we don't train them.
- **Not a general GUI framework.** The UI is tailored to Rustacle's flows; Solid/React + Tailwind, no plugin UIs beyond contributed components.
- **Not a Bash replacement.** We host shells; we don't implement one.

## 7. Inspirations and what we learn from each

| Source | What we take |
|---|---|
| `refs/cc-src` (Claude Code source) | Harness loop shape, tool dispatch, streaming events, `buildTool` factory, tool catalog. See `query.ts`, `QueryEngine.ts`, `Tool.ts`, `tools/*`. |
| [claude-code-from-scratch (FareedKhan-dev)](https://github.com/FareedKhan-dev/claude-code-from-scratch) | Event bus as a first-class architectural primitive; heavy modularity; clear separation of agents, tools, and host. See [`modularity.md`](./modularity.md). |
| [Harness Engineering article](https://levelup.gitconnected.com/building-claude-code-with-harness-engineering-d2e8c0da85f0) | The "harness" mental model — one event loop, cancel-safe futures, dispatch tables. |
| [opencode](https://github.com/anomalyco/opencode) | Provider adapters, session model, local-first posture. |
| `refs/acc` | Modular crate discipline, minimal Tauri shell. |
| `refs/claw-code` (PHILOSOPHY, BRAIN, PARITY) | "Humans direct, agents execute"; parity-sprint checkpoint discipline. Quote from PHILOSOPHY: *"humans set direction; claws perform the labor."* |

## 8. What success looks like

A power user opens Rustacle for the first time, picks Ollama from a menu, grants FS read on their projects directory, types "find all TODOs in src and summarize them", watches the agent think in the side panel, reviews a grep call before it runs, sees the answer — and at no point has opened a file, touched a terminal command they didn't want to, or pasted a key into a text box. When they close the window, every byte stays on their machine.

Then they write a tiny skill in Rust, drop a signed `.wasm` in the plugins dir, and it appears in the tool palette.

Then they switch to Claude on the cloud side for a heavier task, and the agent loop doesn't change.

That's the product.

## 9. The map of docs

See [`README.md`](./README.md) for the reading order.

Thematic groups:

- **What it is**: this file, `glossary.md`.
- **How it's built**: `architecture.md`, `project_structure.md`, `modularity.md`, `tech_stack_2026.md`.
- **How the agent thinks**: `agent_reasoning.md`, `prompts_catalog.md`, `tools_catalog.md`.
- **How it looks and feels**: `ui_ux_manifesto.md`, `ui_simplicity.md`.
- **Integrations**: `mcp_and_models.md`.
- **Cross-platform reality**: `cross_platform.md`.
- **Security**: `security.md`.
- **Visibility**: `observability.md`.
- **Execution plan**: `roadmap.md`.
- **Conventions**: `knowledge_base.md`.
- **Decisions**: `adr/*.md`.

---
*Related: [README](./README.md) · [architecture](./architecture.md) · [agent_reasoning](./agent_reasoning.md) · [ui_ux_manifesto](./ui_ux_manifesto.md) · [modularity](./modularity.md) · [security](./security.md)*
