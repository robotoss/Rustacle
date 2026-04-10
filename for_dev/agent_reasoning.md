# Agent Reasoning

> Audience: engineers working on `plugins/agent`, prompt assembly, tool design, the LLM bridge, or the UI's reasoning panel. Read [`architecture.md`](./architecture.md) and [`glossary.md`](./glossary.md) first.

Companion files: [`prompts_catalog.md`](./prompts_catalog.md) (all prompts verbatim), [`tools_catalog.md`](./tools_catalog.md) (every tool schema).

---

## 1. The Thinking Loop (a.k.a. Harness)

Rustacle runs a classic ReAct-style loop where **every intermediate step is a first-class, typed event streamed to the UI in real time**. This is the most important property of the system and is non-negotiable.

Inspired by the query generator pattern in `refs/cc-src/query.ts` (the `queryLoop` at lines 241-1728) and the "Harness Engineering" article. What we borrow: **one generator loop per turn**, **dispatch table for tools**, **cancel token for Stop**, **content-block-boundary event yielding**, and the split between **concurrent** and **serialized** tools (`refs/cc-src/tools/StreamingToolExecutor`).

```
  ┌────────────┐
  │ User turn  │
  └─────┬──────┘
        ▼
  ┌───────────────────────────────┐
  │  assemble_prompt(ctx)         │  §3 — deterministic 8-layer assembly
  └─────┬─────────────────────────┘
        ▼
  ┌───────────────────────────────┐
  │  LlmProvider::stream(req)     │  host function, plugin never talks HTTP
  └─────┬─────────────────────────┘
        │ ChatDelta stream
        ▼
  ┌───────────────────────────────┐
  │  for chunk in stream:         │
  │    ├─ Text   → ReasoningStep::Thought (partial=true)
  │    ├─ ToolUseStart → buffer
  │    ├─ ToolUseDelta → accumulate args
  │    └─ Done   → flush partial thought (partial=false)
  └─────┬─────────────────────────┘
        ▼
  ┌───────────────────────────────┐
  │  if no tool calls → Answer    │──▶ end of turn
  └─────┬─────────────────────────┘
        │ tool calls present
        ▼
  ┌───────────────────────────────┐
  │  Dispatch                     │
  │    ├─ partition(concurrent)   │
  │    ├─ for each concurrent:    │
  │    │     spawn, emit ToolCall │
  │    ├─ await all concurrent    │
  │    └─ for each serialized:    │
  │          run, emit ToolCall   │
  │          emit ToolResult      │
  └─────┬─────────────────────────┘
        ▼
  append tool observations to conversation → loop back to assemble_prompt
```

### 1.1 Cancel discipline

- One `CancellationToken` per turn (`tokio_util::sync::CancellationToken`).
- The UI Stop button flips it via a Tauri command.
- Every `.await` inside the loop is `tokio::select!`'d against `cancel.cancelled()`.
- In-flight tool calls receive a **child** cancel token; cancelling the turn cancels every child. This matches `refs/cc-src/StreamingToolExecutor.ts:48` where a `siblingAbortController` lets a Bash process abort without killing the whole turn — we keep the primitive and invert the default (Stop ⇒ everything cancels).

### 1.2 Retry discipline

- **Retry transport errors only**: connection reset, HTTP 5xx, SSE disconnect. Exponential backoff, max 3 attempts, all surfaced as `ReasoningStep::Error { retryable: true }` cards before the retry lands.
- **Never retry tool-semantic errors.** A failed shell command is a real observation the model must see. A file-not-found is not a bug the harness fixes; the model reacts to it.

### 1.3 Cost tracker

Every LLM chunk carries token usage deltas; every tool call carries wall time. Both emit `CostSample` on `agent.cost` (`CoalesceLatest` policy — the UI only needs the latest total, not every sample).

---

## 2. Reasoning Event Schema

```rust
// crates/rustacle-ipc/src/events/agent.rs

#[derive(Clone, Debug, Serialize, specta::Type)]
pub struct ReasoningStep {
    pub id: StepId,                   // ulid
    pub parent_id: Option<StepId>,    // for sub-agent trees
    pub turn_id: TurnId,
    pub ts: UnixMillis,
    pub kind: StepKind,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", content = "data")]
pub enum StepKind {
    Thought {
        text: String,
        partial: bool,                // tokens are still arriving
    },
    ToolCall {
        tool: String,                 // e.g. "fs_read"
        args: serde_json::Value,      // validated against ToolSchema::input_schema
        tab_target: Option<TabId>,    // for shell-redirection
    },
    ToolResult {
        tool: String,
        ok: bool,
        summary: String,              // short, rendered in the card
        payload_ref: Option<BlobRef>, // large output → blob store
        duration_ms: u32,
    },
    PermissionAsk {
        capability: Capability,
        decision: Option<PermissionDecision>, // None while pending
    },
    Answer {
        text: String,
        citations: Vec<Citation>,
    },
    Error {
        message: String,
        retryable: bool,
    },
}
```

Every step is published on `agent.reasoning` (`BlockPublisher` policy — losing a step is a bug), persisted to SQLite in the `reasoning_steps` table, and rendered by the UI as a card. `Thought { partial: true }` events stream token-by-token; the UI appends to the current card without re-layout.

### 2.1 Why a `BlobRef` and not inline bytes

Tool outputs like "read this 2 MB log file" or "grep 40k matches" would blow up both SQLite row sizes and the event bus throughput. Rule:

- `summary` ≤ 2 KiB inline (`"read 1823 lines, first 200 bytes: ..."`).
- Full payload lands in `data/blobs/{ulid}.bin`, referenced by `BlobRef(ulid)`.
- The UI fetches the blob only when a user expands the card.
- Retention: blobs older than N days (UI-configurable, default 30) are GC'd on startup; retention is per-turn, so the whole turn goes.

---

## 3. Prompt Assembly

**Prompt assembly is deterministic and layered.** Given identical `TurnContext`, two invocations must produce byte-identical prompts. This is enforced by `insta` golden tests in `plugins/agent/src/prompt/golden_tests.rs`. Non-determinism is a bug.

The pattern echoes `refs/cc-src/constants/prompts.ts::getSystemPrompt` (line 444) — a function that composes layered string components — but Rustacle locks the order and exposes every layer as a named function for testability.

### 3.1 Inputs (`TurnContext`)

```rust
pub struct TurnContext {
    pub turn_id: TurnId,
    pub user_turn: UserMessage,
    pub history: ConversationHistory,

    // From UI state at turn start
    pub model_profile: ModelProfile,
    pub ui_enabled_tools: Vec<ToolId>,
    pub active_tab: TabSnapshot,     // cwd, shell, last N commands
    pub open_tabs: Vec<TabSummary>,
    pub host_os: HostOs,

    // From plugin services
    pub permissions: PermissionView, // what's currently granted
    pub project_docs: ProjectDocs,   // RUSTACLE.md / CLAUDE.md walk
    pub memory:       MemoryView,
    pub selected_files: Vec<PathBuf>,

    // Clock (injected so golden tests can pin it)
    pub now: UnixMillis,
}
```

### 3.2 Principles of prompt construction

These are the hard rules every change to `assemble_prompt` must respect.

1. **Determinism first.** No `HashMap` iteration in prompt output — use `BTreeMap` or sorted vectors. No wall-clock time except via `ctx.now`. No random IDs except via a seeded RNG.
2. **Additive layers, never interleaved.** Each layer is appended once, in a fixed order. No layer may inspect or mutate a previous layer.
3. **Tools the user disabled are invisible to the model.** Filter happens **before** the prompt is built; the model cannot call a tool it does not see.
4. **Env context is a snapshot.** `cwd`, `shell`, `os`, `open tabs` come from the active tab at **turn start**. Switching tabs mid-turn does not change the current turn's prompt.
5. **Memory retrieval is scored against the user turn only**, never against history. This keeps the layer stable across tool loops inside the same turn.
6. **Project docs are walked up once per turn**, not re-walked on every loop iteration. The walk result is cached in `TurnContext`.
7. **Every layer has a header.** Layers are separated by `\n\n## <layer-name>\n\n` so snapshot diffs are readable and so the model can attend to sections.
8. **Budget-aware truncation.** Each layer has a configured max (in characters, converted to tokens via a per-model tokenizer): `SYSTEM_BASE` uncapped, `memory` top-K, `history` trimmed from the middle keeping first-user and last-N, `project_docs` trimmed per-file.
9. **Tool schemas go through the provider's tool-use dialect, not the prompt body.** The `set_tools(...)` call routes to OpenAI's `tools` array or Anthropic's `tools` parameter, never to free text.
10. **Never leak secrets into the prompt.** The LLM router strips known secret values from outgoing requests as defense-in-depth; assembly must not read secrets at all.

### 3.3 Layering (pseudocode — order is law)

```rust
pub fn assemble_prompt(ctx: &TurnContext) -> Prompt {
    let mut p = Prompt::new(ctx.model_profile.tokenizer());

    // 1. Immutable system base: identity, safety posture, output format.
    //    Full text in prompts_catalog.md §1.
    p.push_system_section("system_base", SYSTEM_BASE);

    // 2. Model profile overlay: per-model quirks, tool-use dialect hints,
    //    optional user persona override. Full templates in prompts_catalog.md §2.
    p.push_system_section(
        "model_profile",
        ctx.model_profile.system_overlay(),
    );

    // 3. Environment context derived from the active terminal tab.
    //    Template in prompts_catalog.md §3.
    p.push_system_section("env_context", render_env_context(EnvContext {
        cwd:   ctx.active_tab.cwd.clone(),
        shell: ctx.active_tab.shell.clone(),
        os:    ctx.host_os,
        tabs:  summarize_tabs(&ctx.open_tabs),
        now:   ctx.now,
    }));

    // 4. Tool manifest — ONLY tools the user enabled AND has permission for.
    //    This goes through the provider's native tool-use dialect, not prose.
    let tools: Vec<ToolSchema> = ctx.ui_enabled_tools
        .iter()
        .filter(|id| ctx.permissions.allowed_for_tool(id))
        .map(|id| ToolRegistry::schema(id))
        .collect();
    p.set_tools(tools);

    // 5. Project context: nearest RUSTACLE.md / CLAUDE.md walking up from cwd,
    //    each truncated to its per-file budget.
    for doc in ctx.project_docs.walk_up_from(&ctx.active_tab.cwd) {
        p.push_system_doc(format!("project_doc:{}", doc.rel_path), &doc.body);
    }

    // 6. Selected files: content-block injection for files the user pinned.
    for path in &ctx.selected_files {
        if let Ok(body) = read_with_budget(path, 8 * 1024) {
            p.push_system_file_block(path, &body);
        }
    }

    // 7. Long-term memory: top-K relevant entries scored against the user turn.
    for mem in ctx.memory.relevant_to(&ctx.user_turn, /* top_k = */ 6) {
        p.push_system_memory(&mem);
    }

    // 8. Conversation history.
    p.push_history(&ctx.history);

    // 9. The user turn itself.
    p.push_user(&ctx.user_turn);

    p.finalize()
}
```

**Invariant**: a tool that is not enabled in the UI is not visible to the model at all. The failure mode "the agent called a tool you didn't authorize" is physically impossible.

### 3.4 Cwd-aware and per-tab context

- **Cwd-aware**: layer 3's `cwd` is the **active tab's cwd at turn start**, not the host process `cwd`. Switching tabs takes effect on the next turn.
- **Per-tab shell context**: each tab carries its last N commands + exit codes. Summarized into layer 3 so the agent can reason about "what you just tried".
- **Tool-use redirection**: when the model calls a shell-style tool, the dispatcher reads `tab_target` from the tool args (default: active tab). The UI shows an arrow; the user can reroute by drag. See [`ui_ux_manifesto.md` §3](./ui_ux_manifesto.md).

### 3.5 Golden tests

Located in `plugins/agent/src/prompt/golden_tests.rs`:

```rust
#[test]
fn prompt_is_byte_identical_for_fixed_context() {
    let ctx = fixtures::turn_context_fixture_a();
    insta::assert_snapshot!(assemble_prompt(&ctx).to_string());
}

#[test]
fn changing_cwd_changes_only_env_layer() {
    let a = fixtures::turn_context_fixture_a();
    let b = fixtures::turn_context_fixture_a_with_cwd("/tmp");
    let diff = diff_sections(
        &assemble_prompt(&a).to_string(),
        &assemble_prompt(&b).to_string(),
    );
    assert_eq!(diff.changed_sections, vec!["env_context"]);
}
```

Every PR touching `prompt/` updates at least one snapshot, reviewed in diff.

---

## 4. Tool Dispatch

### 4.1 Tool trait (plugin-internal)

```rust
// plugins/agent/src/tools/mod.rs

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn schema(&self) -> &ToolSchema;

    /// Tool-level prompt contribution (appended to the tool description in the
    /// tool-use dialect — not to the free prose). Keep under 300 tokens.
    fn prompt_addendum(&self) -> &str { "" }

    /// Cheap synchronous validation. Runs before permission check.
    fn validate(&self, args: &Value) -> Result<(), ToolError>;

    fn concurrency(&self) -> Concurrency;

    /// Pure permission check — does this call need capabilities we don't have?
    fn required_capabilities(&self, args: &Value) -> Vec<Capability>;

    async fn call(&self, args: Value, ctx: ToolCtx) -> Result<ToolOutput, ToolError>;
}

pub enum Concurrency {
    /// Safe to run in parallel with other Concurrent tools within the same turn.
    Concurrent,
    /// Must run alone; dispatcher drains all in-flight first.
    Serialized,
}
```

### 4.2 Dispatch table

```rust
pub struct ToolDispatchTable {
    by_name: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolDispatchTable {
    pub fn register<T: Tool + 'static>(&mut self, t: T) {
        self.by_name.insert(t.schema().name.clone(), Arc::new(t));
    }
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.by_name.get(name)
    }
}
```

Plugin init registers every stock tool; user skills are registered at skill-load time via `plugins/skills`.

### 4.3 Dispatch loop (pseudocode)

```rust
async fn dispatch_calls(
    calls: Vec<ModelToolCall>,
    table: &ToolDispatchTable,
    ctx: &HarnessCtx,
) -> Vec<ToolObservation> {
    let (concurrent, serialized): (Vec<_>, Vec<_>) = calls
        .into_iter()
        .partition(|c| matches!(
            table.get(&c.name).map(|t| t.concurrency()),
            Some(Concurrency::Concurrent)
        ));

    // Concurrent: fan out.
    let mut set = JoinSet::new();
    for call in concurrent {
        let tool = table.get(&call.name).unwrap().clone();
        let child_cancel = ctx.cancel.child_token();
        let tctx = ctx.tool_ctx(child_cancel.clone());
        set.spawn(async move {
            emit_step(StepKind::ToolCall { tool: call.name.clone(), args: call.args.clone(), .. });
            let res = tokio::select! {
                r = tool.call(call.args, tctx) => r,
                _ = child_cancel.cancelled() => Err(ToolError::Cancelled),
            };
            (call.id, res)
        });
    }
    let mut observations = drain_joinset(set).await;

    // Serialized: one at a time.
    for call in serialized {
        let tool = table.get(&call.name).unwrap().clone();
        let tctx = ctx.tool_ctx(ctx.cancel.child_token());
        emit_step(StepKind::ToolCall { .. });
        let res = tool.call(call.args, tctx).await;
        observations.push((call.id, res));
    }
    observations
}
```

### 4.4 Catalog

Full schemas for every stock tool (including `buildTool`-style definitions) live in [`tools_catalog.md`](./tools_catalog.md). Summary of what ships at Sprint 4:

| Tool | Concurrency | Capabilities | Notes |
|---|---|---|---|
| `fs_read` | Concurrent | `Fs(read)` | Binary detection, image summary, PDF extract (stretch). Pattern from `refs/cc-src/tools/FileReadTool/FileReadTool.ts`. |
| `fs_write` | Serialized | `Fs(write)` | Size limit, binary guard. Pattern from `FileWriteTool`. |
| `fs_edit` | Serialized | `Fs(write)` | String-replace with uniqueness check. Pattern from `FileEditTool`. |
| `grep` | Concurrent | `Fs(read)` | Ripgrep backend (not shell-escaped). Pattern from `GrepTool`. |
| `glob` | Concurrent | `Fs(read)` | Pattern from `GlobTool`. |
| `bash` | Serialized | `Pty` | Delegates to `plugins/terminal` via a kernel command; does not spawn from wasm. Pattern from `BashTool`. |
| `sub_agent` | Serialized | `LlmProvider` | Spawns a child harness with a bounded budget. Pattern from `AgentTool/runAgent.ts`. |

---

## 5. Harness Engineering Notes

Distilled from `refs/cc-src/query.ts::queryLoop` (lines 241-1728) and the "Harness Engineering" article:

1. **Single generator loop per turn.** The loop is `async fn` returning a stream; the UI subscribes to the stream. No nested loops over loops.
2. **Destructure state at top.** `cc-src` destructures mutable state at loop-top to allow multiple continue paths. We do the same: the loop holds `HarnessState { history, pending_tools, budget, .. }` and every branch mutates the same struct.
3. **Emit early, emit often.** Every state change gets an event before any I/O — users see "Reading file…" card **before** the actual read begins.
4. **Streaming discipline.** `Thought { partial: true }` events flush on **sentence boundaries** (punctuation + space) or every **80 ms**, whichever first. Avoids both char-by-char jitter and perceived batching. Tunable in Settings (`reasoning.stream.flush_ms`).
5. **Budget guardrails.** Each turn has a max-tool-calls, max-duration, and max-tokens budget (UI-configurable). Hitting any emits a `ReasoningStep::Error { retryable: false }` card and ends the turn cleanly.
6. **Sub-agent trees.** A tool may spawn a child harness (`sub_agent`); child steps use the parent's `StepId` as `parent_id`, so the UI renders them as a nested collapsible subtree. Pattern from `refs/cc-src/tools/AgentTool/runAgent.ts`.
7. **No implicit IO in the hot loop.** Logging uses `tracing` (non-blocking subscriber); event emission is via bounded channel.

---

## 6. Multi-Model Support

### 6.1 Host trait

The `LlmProvider` trait lives on the **host**, not inside the `agent` plugin. Plugins are sandboxed WASM — they cannot open sockets. The plugin calls a host function (`llm-stream` in `rustacle.wit`); the host routes to the configured provider.

```rust
// crates/rustacle-llm/src/provider.rs

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;

    async fn stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<BoxStream<'static, Result<ChatDelta, ProviderError>>, ProviderError>;
}

pub struct ProviderCapabilities {
    pub tool_use: bool,
    pub vision:   bool,
    pub streaming: bool,
    pub max_context: u32,
    pub supports_json_mode: bool,
}

pub enum ChatDelta {
    Text { text: String },
    ToolUseStart { id: String, name: String },
    ToolUseDelta { id: String, args_json_chunk: String },
    ToolUseEnd { id: String },
    Usage(TokenCost),
    Done,
}
```

### 6.2 Providers shipped at 1.0

| Provider | Crate | Notes |
|---|---|---|
| OpenAI | `rustacle-llm-openai` | Also covers every OpenAI-compatible server (Ollama, LM Studio, vLLM, llama.cpp-server, local proxies). |
| Anthropic | `rustacle-llm-anthropic` | Tool-use dialect differs enough to warrant a dedicated client. |
| Local autodiscovery | `rustacle-llm-local` | Probes `localhost:11434` (Ollama), `localhost:1234` (LM Studio), etc. at startup; auto-creates model profiles the user can enable with one click. |

### 6.3 Adding a provider

- New crate `rustacle-llm-<name>`.
- `impl LlmProvider`.
- Register in `rustacle-llm::registry::default_providers()`.
- Add to Settings UI "Providers" list (one file in `ui/src/components/settings/ModelProfiles.tsx`).
- **Zero kernel changes.** **Zero prompt changes** (provider-specific overlays live in `ModelProfile::system_overlay()`).

### 6.4 Configuration

100% UI-driven. There is no `~/.rustacle/config.json`. Provider endpoints, API keys, default profiles — all typed controls in the Settings UI, persisted to SQLite by `rustacle-settings`. Secrets via `keyring`. See [`ui_ux_manifesto.md` §1](./ui_ux_manifesto.md).

---

## 7. How a Turn Looks Concretely

End-to-end trace for a single user message:

1. **User types** "find all TODOs in the src dir and summarize" in the chat input.
2. **UI command** `start_turn { text, active_tab_id }` → kernel → `plugins/chat` → `plugins/agent`.
3. **Agent plugin** builds `TurnContext` from host calls (settings, memory, project docs, selected files).
4. **`assemble_prompt(ctx)`** runs; the 8-layer Prompt is serialized for the provider's dialect.
5. **`host.llm_stream(req)`** is called. The host routes to the active profile's provider.
6. **LLM starts streaming text**. Each `Text` delta becomes a `Thought { partial: true }` step on `agent.reasoning`. The UI renders a growing card.
7. **LLM emits `ToolUseStart { name: "grep" }`**. The agent buffers args, emits nothing yet.
8. **`ToolUseEnd`**. Agent validates, checks permissions (still held from earlier grants), emits `ToolCall { tool: "grep", args: { pattern: "TODO", path: "src" } }` step. UI renders a tool-call card.
9. **Dispatcher** runs `grep` (concurrent tool). Ripgrep backend returns 23 matches.
10. **Agent emits `ToolResult { summary: "23 matches in 12 files", payload_ref: Some(..) }`**. UI updates the card.
11. **Loop iteration 2**: new prompt includes the grep result; model streams the summary.
12. **Model emits `Answer { text: "..." }`**. Agent emits final step, ends turn.
13. **UI** renders the answer, keeps the reasoning trail scrollable.
14. **SQLite** persists every step; the trail is replayable from the history view.

Every step is cancellable at every `.await`. Stop button = one token flip = clean unwind.

---
*Related: [README](./README.md) · [architecture](./architecture.md) · [prompts_catalog](./prompts_catalog.md) · [tools_catalog](./tools_catalog.md) · [ui_ux_manifesto](./ui_ux_manifesto.md) · [knowledge_base](./knowledge_base.md) · [glossary](./glossary.md)*
