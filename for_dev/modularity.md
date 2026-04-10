# Modularity — Core vs Plugins, Extension Points, Event Bus

> How Rustacle stays small at the center and rich at the edges. What lives in the kernel, what lives outside, how features are added and removed without recompiling the host.

Heavily inspired by `refs/cc-src` (the `Tool` factory + `query.ts` event yields), [claude-code-from-scratch](https://github.com/FareedKhan-dev/claude-code-from-scratch) (event-bus-centric design, small core with many agents), and the "Harness Engineering" article.

---

## 1. The two-layer model

Rustacle has **exactly two architectural layers**:

1. **Core (host, micro-kernel)** — a small set of Rust crates that the application **cannot run without**. Changes to core are rare and require an ADR.
2. **Plugins (edges)** — everything else. Chat, terminal, agent, memory, skills, FS, providers, MCP, tools. Changes to a plugin affect only that plugin; hot-swappable.

This is the split line:

```
CORE (must exist for the app to start)       │   PLUGINS (everything else)
──────────────────────────────────────────── │  ───────────────────────────────
rustacle-kernel   lifecycle, registry, bus   │   plugins/fs
rustacle-ipc      typed commands/events      │   plugins/terminal
rustacle-plugin-api trait + adapter          │   plugins/chat
rustacle-plugin-wit WIT contracts            │   plugins/agent
rustacle-wasm-host wasmtime runtime          │   plugins/memory
rustacle-settings zero-json store            │   plugins/skills
rustacle-llm      LlmProvider trait          │   plugins/mcp-client
rustacle-app      Tauri binary               │   (user-authored skills)
                                             │
                                             │   PROVIDERS (host-side, selectable per profile):
                                             │   rustacle-llm-openai
                                             │   rustacle-llm-anthropic
                                             │   rustacle-llm-local
```

### 1.1 What lives in core and why

Core holds only what:

- **Every feature depends on** (IPC, event bus, permissions).
- **Must be trusted** (signing verification, capability broker, keyring access).
- **Must be small and audited** (threat surface).

The kernel is ~5k LOC at Sprint 2 and ≤10k LOC at Sprint 8 target. Anything larger than that is a signal to move logic into a plugin.

### 1.2 Why providers are in core crates, not plugins

`rustacle-llm-openai`/`-anthropic`/`-local` live as **host-side crates**, not WASM plugins, for two reasons:

1. **Network + secrets.** Providers need the OS keyring and outbound TLS with hostname pinning; doing this inside WASM would require broad host imports that defeat the sandbox.
2. **Streaming.** `BoxStream` + `CancellationToken` across the WASM boundary is inefficient; host-side keeps allocations low on the hot path.

Providers are selected at runtime **per model profile** in the Settings UI. Adding a new provider is:
- a new `rustacle-llm-<name>` crate,
- `impl LlmProvider`,
- register in `default_providers()`,
- expose in the Settings UI provider list.

Zero kernel changes. Zero WASM boundary crossings beyond the existing `llm-stream` host fn.

### 1.3 Why the agent is a plugin, not core

Putting the agent in core would break the promise that "every feature is replaceable". Today's Thinking loop could look obsolete next year; a user or team should be able to ship their own agent plugin without forking the host. By keeping the agent as a signed WASM plugin, we get:

- **Replaceable reasoning** without recompiling the host.
- **Multiple concurrent agents** (e.g., a research agent + a coding agent) as separate plugin instances with separate permissions.
- **Sandboxed untrusted experimentation** — a user can drop a community agent into the plugins dir and try it without trusting its code.

---

## 2. Extension points

These are the seams where third-party code hooks into Rustacle **without** modifying core.

| Extension point | Mechanism | Audience |
|---|---|---|
| **New plugin** | Signed `.wasm` implementing the WIT `module` interface, dropped in plugins dir | Third-party devs, teams |
| **New tool inside the agent** | Rust code added to `plugins/agent/src/tools/` (requires plugin rebuild) | Agent plugin maintainers |
| **New user skill** | Declarative `skill.toml` + optional handler WASM, loaded by `plugins/skills` | End users |
| **New LLM provider** | New `rustacle-llm-<name>` host crate implementing `LlmProvider` | Rust devs |
| **New MCP server** | Config entry in Settings UI; launched as subprocess by `plugins/mcp-client` | End users / ops |
| **New theme** | CSS custom-property bundle imported via Settings | Anyone |
| **New keybinding preset** | Typed bundle imported via Settings | Anyone |
| **New palette entry** | Declared in plugin's `ModuleManifest::ui_contributions` | Plugin authors |
| **New settings schema** | JSON Schema blob in `ModuleManifest::ui_contributions.settings_schema`; rendered by Settings UI | Plugin authors |
| **New event topic** | Declared by publisher plugin; consumers subscribe by name | Plugin authors |
| **New host function** | Core change — requires ADR, new WIT interface, version bump | Core maintainers only |

---

## 3. The event bus as the central nervous system

Inspired by [claude-code-from-scratch](https://github.com/FareedKhan-dev/claude-code-from-scratch): **the bus is how modules talk to each other**, not direct calls. Direct cross-plugin function calls are forbidden; everything goes through typed topics with explicit backpressure policies.

### 3.1 Why an event bus is the right primitive

- **Decoupling.** Producer and consumer don't know about each other. `plugins/memory` can subscribe to `agent.reasoning` without `plugins/agent` knowing memory exists.
- **Multicast by default.** One reasoning step lights up the UI, gets persisted, lands in memory, and updates the cost badge — one emit, four consumers.
- **Observability for free.** Every topic is a well-known place to tap a probe; `tracing` spans can be hung off publish sites.
- **Replay.** Persisted topics can be replayed for tests, crash-recovery, or session debugging.
- **Hot-swap friendly.** When a plugin unloads, its subscriptions drop without anybody else caring.

### 3.2 Topic registry

Canonical list lives in `architecture.md` §4.6 and is kept in sync with `crates/rustacle-kernel/src/bus/topics.rs`. Highlights:

| Topic | Publisher | Typical subscribers | Policy |
|---|---|---|---|
| `agent.reasoning` | `plugins/agent` | UI, `memory`, SQLite persister, observability exporter | `BlockPublisher` (can't lose steps) |
| `agent.cost` | `plugins/agent` | UI cost badge, budget enforcer | `CoalesceLatest` |
| `agent.turn.started` / `agent.turn.ended` | `plugins/agent` | `chat`, history, analytics | `BlockPublisher` |
| `terminal.output` | `plugins/terminal` | UI xterm widgets | `DropOldest` |
| `terminal.cwd` | `plugins/terminal` | `plugins/agent` (prompt env) | `CoalesceLatest` |
| `terminal.exit` | `plugins/terminal` | `plugins/agent` | `BlockPublisher` |
| `fs.selected` | UI | `plugins/agent`, `chat` | `CoalesceLatest` |
| `fs.change` | `rustacle-kernel` FS watcher | `plugins/agent`, UI file tree | `DropOldest` |
| `permission.ask` | kernel | UI | `BlockPublisher` |
| `permission.changed` | `rustacle-settings` | `PermissionBroker` | `BlockPublisher` |
| `plugin.loaded` / `plugin.unloaded` | kernel | UI plugin list, analytics | `BlockPublisher` |
| `mcp.tool.available` | `plugins/mcp-client` | `plugins/agent` (tool registry merge) | `CoalesceLatest` |
| `telemetry.span` | kernel tracing bridge | `observability` exporter | `DropOldest` |

### 3.3 Event flow: a full turn

```
user types message
    │
    ▼
UI ── start_turn (IPC cmd) ─▶ plugins/chat ── turn.started ─▶ plugins/agent
                                                                    │
                                                          assemble_prompt
                                                                    │
                                                          host.llm_stream
                                                                    │
                          ┌─────────── agent.reasoning (Thought) ───┤
                          │                                         │
                          ▼                                         │
              UI ◀── ReasoningCard                              tool call
                                                                    │
                          ┌─────────── agent.reasoning (ToolCall) ──┤
                          │                                         │
                          ▼                                         ▼
              UI ◀── ToolCallCard                         dispatch to tool
                                                                    │
                                                   (e.g. bash → terminal)
                                                                    │
                          ┌─────────── terminal.output ─────────────┤
                          ▼                                         │
              UI xterm widget                                       │
                                                                    ▼
                          ┌─────────── agent.reasoning (ToolResult)─┤
                          ▼                                         │
              UI ◀── updated card                              loop back
                                                                    │
                          ┌─────────── agent.reasoning (Answer) ────┘
                          ▼                                          
              UI ◀── final answer                                    
                          │                                          
                          ▼                                          
              turn.ended ─▶ chat, history, analytics
```

Every step is a typed event on a well-known topic. No direct cross-plugin calls.

---

## 4. Many-agent patterns

Because the agent is a plugin and providers are host-selectable, Rustacle supports several multi-agent patterns out of the box.

### 4.1 Sub-agent (child harness)

One loop spawns a child for a bounded task. Pattern from `refs/cc-src/tools/AgentTool/runAgent.ts`. In Rustacle this is the `sub_agent` tool — see `tools_catalog.md` §7. Child steps carry the parent's `StepId` as `parent_id`, so the UI renders nested collapsible subtrees.

### 4.2 Multiple concurrent plugin instances

The registry allows the same agent plugin binary loaded as two **instances** with different permissions, model profiles, and UI panels:

- `agent.coder` — full FS/Pty, local model, no net.
- `agent.researcher` — web search tool via MCP, cloud model, no FS write.

A Settings UI toggle lets the user switch the active instance per tab, or run both in parallel.

### 4.3 Delegation bus pattern (inspired by claude-code-from-scratch)

Agents can post "task requests" on a dedicated `agent.delegate` topic. A specialized agent subscribes to tasks matching its profile and replies on a reply-topic. This enables **declarative agent composition** without hardcoding caller/callee relationships.

### 4.4 User-as-agent

The event bus does not care whether a `ToolCall` came from a model or from a human. The command palette can dispatch tool calls directly, and they appear in the reasoning panel as `ToolCall { initiator: User }`. This gives users the same visibility and audit trail as an agent.

---

## 5. What goes in core vs plugin — decision framework

When proposing new functionality, ask **in order**:

1. **Does every install need this?** If no → plugin.
2. **Does it need the keyring, TLS, or spawning a process?** If yes → host crate (core or provider-style), not WASM plugin.
3. **Is it a new communication primitive?** (bus, IPC) → core.
4. **Is it a new permission type?** → core (requires ADR).
5. **Is it user-visible behavior, a tool, a UI surface?** → plugin.
6. **Is it a data integration (API, database, file format)?** → plugin.
7. **Is it performance-critical on the hot path, does it need `Bytes` zero-copy?** → can be core with a small stable surface.

Default answer: **plugin**.

---

## 6. Plugin contract evolution

The WIT file in `crates/rustacle-plugin-wit/wit/rustacle.wit` is versioned with `package rustacle:plugin@X.Y.Z`.

- **Minor bump (additive)**: new host fn, new optional field, new capability variant. Old plugins keep working.
- **Major bump (breaking)**: renamed field, removed fn, changed signature. Requires an ADR, a migration window (one release ships both versions), and updates to every first-party plugin.

The kernel checks the plugin's declared `package` version at load time and refuses incompatible majors with a visible error.

---

## 7. User-authored skills

`plugins/skills` is the user extension surface for people who don't want to write a full plugin.

A skill is a directory:

```
my-skill/
├── skill.toml            # name, description, input schema, capabilities, concurrency
├── handler.wasm          # optional: a small component with a run() function
└── README.md             # optional user-facing notes, shown in Settings
```

`skill.toml` example:

```toml
[skill]
name = "jira_ticket"
description = "Fetch a Jira ticket by ID and return its summary and description."
version = "0.1.0"

[skill.input_schema]
type = "object"
required = ["ticket_id"]
properties.ticket_id = { type = "string" }

[skill.capabilities]
net = { hosts = ["jira.example.com"] }
secret = { key = "jira_token" }

concurrency = "concurrent"
```

The user enables a skill from Settings → Skills; it appears in the agent's tool list on the next turn. Skills are sandboxed exactly like first-party plugins.

---

## 8. What this buys the user

- **Try without commit.** Drop a signed plugin in; click install; revoke cleanly.
- **Replace without rewrite.** Swap the agent plugin for a community one; your tools and UI don't change.
- **Extend in Rust or declarative.** Skills cover the declarative case; full plugins cover the complex case.
- **Audit, not inspect.** Every extension goes through signing + capability broker + audit log.
- **Forward compat.** Versioned WIT means a plugin built today keeps working across minor host bumps.

---
*Related: [README](./README.md) · [concept](./concept.md) · [architecture](./architecture.md) · [project_structure](./project_structure.md) · [mcp_and_models](./mcp_and_models.md) · [security](./security.md)*
