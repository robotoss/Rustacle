# Observability — Logging, Tracing, Token Accounting

> Nothing the agent does is invisible. Every turn, every tool, every LLM call, every plugin lifecycle event, every permission decision is logged, spanned, traceable, and replayable. This file says how.

---

## 1. Goals

1. **Reproducibility.** From a trace ID, any engineer can reconstruct what happened in a turn.
2. **Debuggability.** Every `Result::Err` that reached the user has a full span chain in the logs.
3. **Cost clarity.** Token usage and wall-clock cost are visible in real time and attributable per tool / per turn / per profile.
4. **Privacy.** All of the above happen without leaking secrets.

---

## 2. Logging with `tracing`

### 2.1 Rules

- **One crate, one target.** Every crate sets `const TARGET: &str = "rustacle::<crate>"` so filters work cleanly (`RUSTACLE_LOG=rustacle::agent=debug`).
- **Structured fields, not string interpolation.**
- **Span per logical unit**: turn, command, tool call, plugin load, permission check, LLM request, MCP call.
- **Never `println!`.** A `println!` in non-test code is a review reject.
- **Redaction layer is mandatory.** The subscriber pipeline passes every event through a `Redactor` that scrubs known secret values (from keyring) and PII-flagged fields.

### 2.2 Level meaning (project-wide)

| Level | Use for | Example |
|---|---|---|
| `error` | The user will see this or should. | `provider request failed after retries` |
| `warn` | Degraded mode reached. | `subscriber lagging on agent.reasoning, coalescing` |
| `info` | Lifecycle events. | `plugin loaded`, `turn started`, `hot-swap complete` |
| `debug` | Dev-time insights. | `prompt assembled layer=env_context len=847` |
| `trace` | Firehose — off in prod. | per-chunk LLM deltas |

### 2.3 Span taxonomy

```
root: app_session { session_id }
├── command { trace_id, name="start_turn" }
│   └── turn { turn_id, profile_id, tab_id }
│       ├── prompt_assembly { layer_count, total_tokens }
│       ├── llm_request { provider_id, model, tokens_in }
│       │   └── llm_chunk (trace-level)
│       ├── tool_dispatch { concurrent_count, serialized_count }
│       │   ├── tool_call { tool="grep", tool_call_id }
│       │   │   └── permission_check { capability_key }
│       │   └── tool_call { tool="fs_read", … }
│       └── turn_end { outcome="answer", duration_ms, tokens_out, cost_usd }
├── plugin_event { plugin_id, topic }
└── hot_swap { plugin_id, from, to, duration_ms }
```

Every span carries `trace_id` (ulid) as a field so filtering is `rg <trace_id> logs/`.

### 2.4 Subscriber pipeline

```
tracing events
    │
    ▼
Redactor layer (strips secrets, PII-flagged fields)
    │
    ├── fmt layer → stderr (dev) / file (prod, rotated daily)
    ├── file layer → data_dir/logs/rustacle.jsonl (JSON, ship-friendly)
    ├── bus bridge → telemetry.span topic (for the observability exporter plugin)
    └── (optional, opt-in) OTLP exporter
```

Log files under `data_dir/logs/`: `rustacle.log` (pretty), `rustacle.jsonl` (machine), `audit.jsonl` (security-relevant events only).

### 2.5 Redaction

`Redactor` does three things:

1. **Strips known secret values** from event fields. The keyring exposes a "redaction list" of cached plaintext values (kept only in-memory, never logged themselves) that the redactor matches against as substrings. Matches are replaced with `••••`.
2. **Masks PII-flagged fields.** Fields tagged with `#[tracing::field(secret)]` or named `password`, `token`, `api_key`, `authorization`, `secret`, `cookie` are redacted by name.
3. **Truncates huge values**. Fields longer than 4 KiB become `<truncated 12345 bytes>`.

---

## 3. Trace IDs, everywhere

- Every IPC command generates a `trace_id = ulid()` at entry.
- The trace ID propagates through spans as a field.
- The trace ID is attached to `RustacleError::Internal`, surfacing to the UI as a copyable string in error dialogs.
- The trace ID is attached to every event published on the bus, so event persistence retains the lineage.

User-facing error dialog:

```
Something went wrong while calling the LLM provider.
Trace: 01HYBAZRZN85ZP8V0G2MZF9DXS      [ Copy ]
```

Engineer reproduces: `rg 01HYBAZRZN85ZP8V0G2MZF9DXS data/logs/` → full span tree.

---

## 4. Token accounting

### 4.1 Where numbers come from

Providers emit `ChatDelta::Usage(TokenCost)` at stream end (and sometimes mid-stream for Anthropic). The harness publishes each on `agent.cost`.

```rust
pub struct TokenCost {
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub cost_usd: Option<f64>,        // None for local models
    pub provider_id: String,
    pub model: String,
    pub turn_id: TurnId,
    pub attribution: CostAttribution, // which tool call or prompt-assembly step
}

pub enum CostAttribution {
    TurnPrompt,
    ToolCall { tool: String, call_id: String },
    SubAgent { parent_turn: TurnId, child_turn: TurnId },
}
```

### 4.2 Where numbers show up

1. **Cost badge** — top-right of the Agent Panel, always visible, updated on every sample (`CoalesceLatest`).
2. **Per-turn rollup** — clicking the badge expands into a breakdown by tool.
3. **Settings → Usage** — historical charts per profile, per provider, per day.
4. **Per-tool attribution** — every tool-call card carries its own cost line.
5. **Sub-agent aggregation** — child costs roll up into the parent card.
6. **Budget enforcement** — when a per-turn budget is set, exceeding it emits `ReasoningStep::Error { retryable: false }` and ends the turn.

### 4.3 Determinism and audit

Every LLM call is logged in the `llm_audit` SQLite table with: `ts`, `turn_id`, `profile_id`, `provider`, `model`, `tokens_in`, `tokens_out`, `duration_ms`, `cost_usd`, `outcome`. The Usage view is a query over this table — no magic.

### 4.4 Local models

Local models emit `tokens_in/out` but `cost_usd = None`. The badge shows `$0.00` with a small "local" marker. Users who want to approximate local cost (electricity, depreciation) can set a custom per-token rate per profile in Settings; this is a display-only multiplier and does not affect audit records.

---

## 5. Event persistence and replay

Every event on `agent.reasoning`, `agent.cost`, `permission.ask`, `permission.changed`, `plugin.loaded`, `plugin.unloaded`, `mcp.tool.available` is persisted to SQLite (topics declared with the `Persisted` marker in the bus config).

### 5.1 Replay UI

Settings → History → Session X → Turn Y → "Replay". Plays the reasoning stream back in real time, in simulated mode (no actual tool calls). Useful for:

- Post-mortem debugging.
- Demoing a session to a colleague.
- Re-rendering an old turn under a new UI theme.

### 5.2 Replay-as-dry-run

A user can open an old turn and click "Re-run from here". The harness reuses the persisted `TurnContext`, re-calls the current active profile's model, and produces a **new** turn that can be compared side-by-side against the original. Enables agent A/B testing.

---

## 6. Metrics

For power users and team deployments, `metrics` + `metrics-exporter-prometheus` exposes a local Prometheus endpoint (opt-in in Settings → Observability → Metrics). Default metrics:

| Metric | Type | Labels |
|---|---|---|
| `rustacle_turns_total` | counter | `profile`, `outcome` |
| `rustacle_turn_duration_seconds` | histogram | `profile` |
| `rustacle_tokens_in_total` | counter | `profile`, `provider`, `model` |
| `rustacle_tokens_out_total` | counter | `profile`, `provider`, `model` |
| `rustacle_cost_usd_total` | counter | `profile`, `provider` |
| `rustacle_tool_calls_total` | counter | `tool`, `outcome` |
| `rustacle_tool_duration_seconds` | histogram | `tool` |
| `rustacle_permission_decisions_total` | counter | `capability_kind`, `decision` |
| `rustacle_plugin_state` | gauge | `plugin_id`, `state` |
| `rustacle_bus_lag_events` | counter | `topic`, `subscriber` |
| `rustacle_panic_total` | counter | `crate` |

---

## 7. OpenTelemetry export (opt-in)

Settings → Observability → OTLP Endpoint (URL + optional bearer token in keyring). When enabled:

- `tracing` spans are forwarded via `opentelemetry-otlp` to the configured endpoint.
- Span fields pass through the same `Redactor` as logs.
- The UI shows a live "exporting to <host>" indicator.
- Disabling is one click and takes effect immediately.

---

## 8. Crash reporting (opt-in)

Settings → Observability → Crash reports (on/off). When on:

- `std::panic::set_hook` captures panics, writes a report to `data_dir/crashes/`, and prompts the user on next launch: "Send report?"
- The report contains: panic message, backtrace, trace ID, Rustacle version, OS, a summary of the last turn (redacted). It never contains: secrets, file contents, prompts, tool outputs, user messages.
- Sending is via Sentry (opt-in endpoint) or an email to a user-configured address.

---

## 9. Audit views in Settings

One page per concern, all powered by SQLite queries:

- **Usage** — per-profile, per-provider token/cost charts (§4).
- **Plugins** — load/unload timeline, signature verification history.
- **Permissions** — grants, revocations, denials, with plugin attribution.
- **Shell execs** — every command run via `bash`, initiator, duration, exit code, bytes.
- **MCP** — every MCP call and server lifecycle event.
- **LLM calls** — every provider round-trip.
- **Errors** — every `Internal` error surfaced to the UI, clickable to open the full span tree.

---

## 10. Privacy guarantees (recap)

- Nothing observable leaves the machine by default.
- Every export path is opt-in per endpoint.
- Every export path runs through the `Redactor`.
- Secrets never reach logs, spans, exports, crash reports, or telemetry.
- "Delete all my data" in Settings wipes DB, blobs, logs, crashes, metrics history.

---
*Related: [README](./README.md) · [architecture](./architecture.md) · [security](./security.md) · [agent_reasoning](./agent_reasoning.md) · [knowledge_base](./knowledge_base.md)*
