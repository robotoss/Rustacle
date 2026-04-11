# Knowledge Base

> Audience: anyone writing Rust, TS, or prompts in this repo. Skim before your first PR; re-read the relevant section when you hit the pattern.

---

## 1. Rust Memory Management

### 1.1 Ownership across async boundaries

`.await` points are yield points. Anything held across one lives until the next poll — sometimes for seconds. Guards held across `.await` are the #1 cause of async deadlocks in this codebase.

**Rule**: lock → clone-out → drop guard → await.

```rust
// WRONG — holds the guard across .await
async fn bad(state: Arc<RwLock<Registry>>, id: PluginId) -> Result<()> {
    let guard = state.write().await;
    guard.get(&id)?.do_something_async().await?; // ← deadlock risk
    Ok(())
}

// RIGHT — guard dropped before await
async fn good(state: Arc<RwLock<Registry>>, id: PluginId) -> Result<()> {
    let plugin = {
        let guard = state.read().await;
        guard.get(&id).cloned()
    }; // guard dropped here
    plugin.do_something_async().await?;
    Ok(())
}
```

Prefer `tokio::sync::RwLock` / `Mutex` over `std::sync::*` when the critical section may yield. `parking_lot` is fine for short, non-async critical sections only.

### 1.2 Task ownership

Every `tokio::spawn` has an owner. Orphan tasks are leaks.

```rust
// WRONG — fire-and-forget, no cancellation path
tokio::spawn(async move { do_thing().await });

// RIGHT — owned by the kernel's JoinSet
kernel.tasks.spawn(async move { do_thing().await });

// ALSO RIGHT — cooperative cancellation
let token = kernel.shutdown.child_token();
kernel.tasks.spawn(async move {
    tokio::select! {
        _ = token.cancelled() => {},
        _ = do_thing() => {},
    }
});
```

Kernel shutdown awaits `tasks.shutdown()`. A task that can't see the cancel token blocks shutdown.

### 1.3 `Arc` / `Weak` patterns

- `Arc<T>` for shared read-mostly state.
- `Arc<RwLock<T>>` only when you can articulate **why** `RwLock` beats `Mutex` for this workload (many concurrent readers, few writers).
- Break cycles with `Weak`. The plugin registry holds `Weak<PluginInstance>` in its topic subscriptions so unloading a plugin isn't blocked by dangling subscribers.

```rust
// Registry owns the Arc.
pub struct Registry {
    instances: DashMap<PluginId, Arc<PluginInstance>>,
}

// Subscribers observe via Weak.
pub struct Subscription {
    instance: Weak<PluginInstance>,
}

impl Subscription {
    fn deliver(&self, event: Event) {
        if let Some(instance) = self.instance.upgrade() {
            instance.on_event(event);
        } // otherwise the instance is gone; drop the subscription lazily
    }
}
```

### 1.4 Channel-first concurrency

Default to message passing. The event bus exists precisely so plugins don't reach for shared mutexes.

- **Bounded** channels by default. An unbounded channel is a latent OOM — if you need one, comment why backpressure is impossible.
- `tokio::sync::broadcast` for fan-out, `mpsc` for work queues, `oneshot` for reply channels.
- Backpressure policy on every bus topic is declared — see [`architecture.md` §4.6](./architecture.md).

### 1.5 Contention patterns

- If a mutex shows up in a profiler, **shard** it before making it finer-grained. `dashmap` is the usual shortcut.
- Reasoning steps are append-only: use a per-turn `Vec` guarded by the turn's own lock, never a global log mutex.
- Plugin registry uses `DashMap` keyed by `PluginId`; hot-swap takes the single relevant shard's write lock, not a global one.

### 1.6 Zero-copy where it matters

- Event payloads use `Bytes` (from `bytes` crate) so fan-out across the bus doesn't clone.
- Large blobs travel as `BlobRef` (index into the blob store), not inline.
- Prompt assembly uses `Cow<str>` for static fragments.

---

## 2. Error Handling Across the TS/Rust Bridge

### 2.1 Typed errors, always

Every IPC command returns `Result<T, RustacleError>` where `RustacleError` is a **tagged enum** defined in `rustacle-ipc` and exported via Specta.

```rust
// crates/rustacle-ipc/src/errors.rs
#[derive(thiserror::Error, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", content = "data")]
pub enum RustacleError {
    #[error("permission denied: {capability}")]
    PermissionDenied { capability: String, plugin: String },

    #[error("plugin not found: {id}")]
    PluginNotFound { id: String },

    #[error("invalid input: {field}: {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("plugin trap: {0}")]
    PluginTrap(String),

    #[error("llm provider: {provider}: {reason}")]
    LlmProvider { provider: String, reason: String, retryable: bool },

    #[error("internal: {message}")]
    Internal { message: String, trace_id: String },
}
```

On the TS side, exhaustive match:

```ts
import type { RustacleError } from "../bindings";

function render(err: RustacleError): string {
    switch (err.kind) {
        case "PermissionDenied":
            return `Plugin ${err.data.plugin} needs permission: ${err.data.capability}`;
        case "PluginNotFound":
            return `Plugin ${err.data.id} is not loaded`;
        case "InvalidInput":
            return `${err.data.field}: ${err.data.reason}`;
        case "PluginTrap":
            return `Plugin crashed: ${err.data}`;
        case "LlmProvider":
            return `${err.data.provider}: ${err.data.reason}`;
        case "Internal":
            return `Something went wrong (trace ${err.data.trace_id})`;
        // no default — TS exhaustiveness will catch missing arms
    }
}
```

A default branch is a code smell. `tsc --strict` + `noFallthroughCasesInSwitch` catches missing arms.

### 2.2 User-facing vs internal

- `InvalidInput` / `PermissionDenied` → actionable UI messages pointing at the specific control.
- `LlmProvider { retryable: true }` → "Retry" button + live retry indicator.
- `Internal` → generic "Something went wrong" + a **trace ID** the user can copy. Full error lives in `tracing` logs.
- **Never** surface a `Debug`-formatted Rust error. `format!("{err:?}")` in UI code is a bug.

### 2.3 `thiserror` in crates, `anyhow` only at `main`

- Library crates use `thiserror` with explicit variants.
- The binary crate may use `anyhow` at the very top of `main()` for startup errors that will never be matched on.
- Conversion from internal `KernelError` to `RustacleError` happens at the IPC boundary, explicitly, with loss of detail documented.

### 2.4 Trace IDs

Every IPC command opens a `tracing` span with a `trace_id = ulid()`. The same ID lands in every log line inside the span and in the `Internal` variant if one bubbles up. Users report the trace ID; engineers `rg <id>` in logs.

---

## 3. DX Guidelines

### 3.1 Logging

- **Use `tracing`**, not `println!` or `log`.
- **Structured fields, not interpolation**:

```rust
// WRONG
tracing::info!("plugin {} loaded version {}", id, v);

// RIGHT — searchable, machine-parseable
tracing::info!(plugin.id = %id, plugin.version = %v, "plugin loaded");
```

- **Every turn, command, and plugin lifecycle event opens a span.** Spans carry `turn_id`, `plugin_id`, `trace_id` as fields.
- **Log levels**:
  - `error` = user will see it.
  - `warn` = degraded mode hit.
  - `info` = lifecycle events.
  - `debug` = dev insights.
  - `trace` = firehose; off in production.

### 3.2 Feature flags

- Prefer **runtime toggles via Settings UI** over compile-time `#[cfg(feature = ...)]` for anything user-visible.
- Compile-time features reserved for platform-specific paths (`cfg(windows)`) and optional heavyweight deps.
- Every Cargo feature lives in `Cargo.toml` with a one-line comment explaining it. A feature without a comment is deleted in review.

### 3.3 Testing layers

- **Unit** — pure functions; prompt assembly (`insta`), path canonicalization, scope matching.
- **Integration** — kernel + one plugin, no Tauri window. Spawned via `tests/kernel/`.
- **Contract** — per-plugin WIT binding tests verifying manifest and error mapping.
- **E2E** — full Tauri app via Playwright + `tauri-driver`; reserved for UI-critical flows (turn cancellation, permission dialog, import/export round-trip).
- **Property** — `proptest` for the VT parser, path canonicalizer, backpressure-policy invariants.
- **Fuzz** — `cargo-fuzz` for prompt assembler and WIT deserializers; nightly CI runs.

**Golden prompt tests are mandatory** for any change to `plugins/agent/src/prompt/`. See `agent_reasoning.md` §3.5.

### 3.4 Doc comments

- Every `pub` item in `rustacle-plugin-api` has a doc comment with an example.
- Every `unsafe` block has a `// SAFETY:` comment explaining the invariant.
- `cargo doc --no-deps` must build without warnings in CI.

### 3.5 Commit discipline

- Commits are small and scoped. A commit that touches the kernel and `ui/` in the same diff requires justification in the message.
- Commit messages follow "conventional commits" style: `feat(kernel): …`, `fix(agent): …`, `docs(for_dev): …`.
- Never `git push --force` on `main`. Force pushes on topic branches are fine.

### 3.6 Review checklist

- [ ] No new `.unwrap()` / `.expect()` in non-test code without a `// SAFETY:` / `// INVARIANT:` comment.
- [ ] No new IPC type hand-written in TS.
- [ ] No new setting without a UI control (Zero-JSON rule).
- [ ] New `.await` points reviewed for held guards across them — see §1.1.
- [ ] New `tokio::spawn` has an owner — see §1.2.
- [ ] Prompt-affecting changes include an updated `insta` snapshot, reviewed in diff.
- [ ] New capability declarations wired through the `PermissionBroker`, not ad-hoc `if allowed`.
- [ ] New public trait items in `rustacle-plugin-api` have doc comments with examples.
- [ ] CI green: tests, clippy, fmt, deny, bindings regen.

---

## 4. Security & Threat Model

### 4.1 Attack surfaces and mitigations

Full STRIDE analysis. Updated per sprint.

| Surface | Threat class | Threat | Mitigation |
|---|---|---|---|
| WASM plugin | Spoofing | Unsigned plugin impersonates a trusted one | Ed25519 signatures verified at load against `keys/trusted_plugin_keys.toml` |
| WASM plugin | Tampering | Signed plugin altered after signing | Signature covers the whole `.wasm`; re-verify on every load |
| WASM plugin | Elevation | Escapes sandbox via wasmtime bug | wasmtime hardened config (fuel, memory, no ambient auth, narrow host imports), version kept current |
| WASM plugin | Denial | Infinite loop pins a core | Fuel metering traps; kernel marks instance unhealthy and suspends |
| FS plugin | Tampering | Symlink escape via TOCTOU | Canonicalize before scope check; re-check after open (`openat`) on Unix; reject TOCTOU windows |
| Permission broker | Elevation | Cached grant persists after user revokes | `invalidate()` called on every Settings change; cache hits still logged |
| Permission broker | Elevation | Parameterized capability (path scope) cached against canonicalized scope, but mismatched prefix match | Scope match uses canonical prefix + segment-boundary test (`/home/k` ≠ `/home/kate`) |
| LLM provider | Information disclosure | Prompt injection exfiltrates secrets via a tool call | Secrets never enter prompts; LLM router strips known secret values from outgoing payloads; user sees every tool call before effects |
| LLM provider | Spoofing | Evil endpoint impersonates trusted provider | TLS + pinned hostnames in provider config; keyring-stored API key |
| Settings import | Elevation | Malicious export grants hostile capabilities | Import UI shows typed diff; every capability grant requires explicit user click per plugin |
| Settings import | Tampering | Import modifies theme to exfiltrate data | Themes are CSS custom properties only, not arbitrary JS |
| Logs | Information disclosure | Secrets printed in `tracing` output | `SecretString` redacts on `Debug`; `tracing` fields go through a `Redactor` layer |
| Updater | Spoofing | Fake update server ships a backdoored build | Tauri updater signed manifests; separate key from plugin-signing |
| Host fns | Elevation | A sandboxed plugin calls a host fn with inputs that triggers host RCE | Host fn inputs validated and bounded; no shell interpolation; no filesystem walk outside scope |
| Terminal | Elevation | Shell command exfiltrates via env | PTY inherits scoped env; `TERM` / `PATH` preserved, secret env vars never forwarded |

### 4.2 FS path sandboxing — concrete rules

```rust
pub fn check_fs(scope: &PathScope, req: &Path, mode: FsMode) -> Result<PathBuf, ModuleError> {
    // 1. Canonicalize. Resolves symlinks, normalizes '..'.
    let canonical = req.canonicalize()
        .map_err(|e| ModuleError::Denied(format!("canonicalize failed: {e}")))?;

    // 2. Scope match by canonical prefix AND path-segment boundary.
    let scope_canon = scope.canonical();
    if !is_path_prefix_with_boundary(&canonical, &scope_canon) {
        return Err(ModuleError::Denied(
            format!("{} is outside scope {}", canonical.display(), scope_canon.display())
        ));
    }

    // 3. Mode check.
    if mode == FsMode::ReadWrite && scope.mode == FsMode::Read {
        return Err(ModuleError::Denied("scope is read-only".into()));
    }

    Ok(canonical)
}

fn is_path_prefix_with_boundary(path: &Path, prefix: &Path) -> bool {
    let mut p = path.components();
    for comp in prefix.components() {
        if p.next() != Some(comp) {
            return false;
        }
    }
    // must end at a segment boundary: next component is None or a real segment,
    // never a partial string like "kate" overlapping "k"
    true
}
```

TOCTOU: on Unix, prefer `openat` family via `rustix` so the scope check and open share a directory FD.

### 4.3 Secrets

- Secrets live in the OS keyring via the `keyring` crate. The host exposes a narrow `secret-get` function per plugin id, scoped.
- Secrets are wrapped in `SecretString` (`secrecy` crate); `Debug` is redacted.
- Secrets never appear in logs, never in `Debug` output, never in serialized state dumps.
- Settings export **excludes** secrets; import prompts the user to re-enter them.
- No secret is ever put into a prompt. The LLM router additionally scans outgoing requests for known secret values as defense-in-depth.

### 4.4 WASM host posture

- **Fuel metering** enabled on untrusted plugins; runaway loops trap.
- **Memory limit** per instance (default 64 MiB, per-plugin override in manifest).
- **Host function set** is small, audited, documented in `project_structure.md`. Changes require an ADR.
- **One Store per instance** — cross-plugin memory isolation is enforced by wasmtime.
- **No WASI by default.** We provide a **custom** interface via `rustacle.wit`; adding a WASI world is a deliberate decision with capability implications.

### 4.5 Data retention & privacy

- Reasoning steps stored in SQLite; retention window UI-configurable (default 30 days).
- Blobs under `data/blobs/` GC'd on startup per same retention.
- Memory entries user-managed via Settings; "Forget" wipes entry and GC's associated blobs.
- Telemetry (OTLP, Sentry) **off by default**; user must explicitly opt in per endpoint.
- Every export path (import/export settings, crash report, telemetry span) excludes secrets and redacts known PII fields.
- "Export all my data" and "Delete all my data" are Settings buttons (GDPR-friendly, Zero-JSON compatible).

---

## 5. Common anti-patterns (don't do this)

| Anti-pattern | What to do instead |
|---|---|
| `if user_allowed(cap) { do_it() }` | Go through the `PermissionBroker`. |
| `Arc<Mutex<HashMap>>` as a reflex | Consider `DashMap` or a channel-owned actor. |
| `tokio::spawn(async move { … })` without a handle | Use `kernel.tasks.spawn(…)`. |
| `format!("{err:?}")` into user-facing strings | Map to `RustacleError` variant; let the UI render. |
| `HashMap` iteration in prompt assembly | `BTreeMap` or sorted vector — determinism. |
| `let data = tokio::fs::read(...)` inside a tool without permission check | Always call the host fn; never directly. |
| Hand-writing a TS type that shadows `bindings.ts` | Import from `bindings.ts`; the types are generated. |
| Adding a setting with no UI control | Ship the control in the same PR. |
| `Instant::now()` in prompt assembly | Use `ctx.now` (injected). |
| `unwrap()` on a `Mutex::lock()` | Locks in `tokio::sync::*` never poison; but use `.expect("lock never poisoned")` in rare std cases with a comment. |

---

## 5.1 CI / tooling gotchas

| Gotcha | Fix |
|---|---|
| `cargo-deny` v0.16+ removed `[advisories]` per-field severity keys | Don't use `vulnerability`, `unmaintained`, `unsound` etc. Leave `[advisories]` minimal; use `ignore = [...]` for suppressions. See [PR #611](https://github.com/EmbarkStudios/cargo-deny/pull/611). |
| PowerShell does not support `&&` operator | Use `;` to chain commands: `cd ui; npm install; cd ..` |
| `cargo run -p rustacle-app` fails with "could not determine which binary" | Add `default-run = "rustacle-app"` to the `[package]` section |
| Tauri `devUrl` causes ERR_CONNECTION_REFUSED on `cargo run` | Remove `devUrl`/`beforeDevCommand` from `tauri.conf.json`; use `frontendDist` only. Dev server is for `cargo tauri dev` only. |

---

## 6. Quick references

- **Glossary** — [`glossary.md`](./glossary.md).
- **Prompts** — [`prompts_catalog.md`](./prompts_catalog.md).
- **Tools** — [`tools_catalog.md`](./tools_catalog.md).
- **Crate layout** — [`project_structure.md`](./project_structure.md).
- **WIT contract** — [`architecture.md` §4.2](./architecture.md).
- **Event topics** — [`architecture.md` §4.6](./architecture.md).
- **Review checklist** — §3.6 above.

---
*Related: [README](./README.md) · [architecture](./architecture.md) · [agent_reasoning](./agent_reasoning.md) · [ui_ux_manifesto](./ui_ux_manifesto.md) · [tech_stack_2026](./tech_stack_2026.md) · [glossary](./glossary.md)*
