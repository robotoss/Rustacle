# Architecture

> Audience: engineers about to touch the kernel or write a plugin. Read this **and** [`project_structure.md`](./project_structure.md) before opening a PR against `rustacle-kernel`, `rustacle-plugin-api`, or `rustacle-wasm-host`.

## 1. System Shape

Rustacle is a **micro-kernel** on Rust/Tauri. The kernel is deliberately small: it owns lifecycle, IPC routing, permissions, and the event bus. Everything the user perceives as "a feature" (chat, terminal, file manager, agent) ships as a **plugin** — a signed **WebAssembly Component** by default, with a whitelisted native fallback only for capabilities WASI cannot yet express (today: PTY spawning).

```
 ┌──────────────────────── Tauri Window (TS/UI) ─────────────────────────┐
 │  Command Palette · Terminal Tabs · Agent Panel · Settings UI          │
 └──────────────▲──────────────────────────────────────▲─────────────────┘
                │  tauri-specta (typed commands)       │  typed events
                │  generates bindings.ts                │
 ┌──────────────┴──────────────────────────────────────┴─────────────────┐
 │                          rustacle-kernel                              │
 │                                                                       │
 │   ┌───────────┐   ┌────────────┐   ┌────────────┐   ┌─────────────┐   │
 │   │ Lifecycle │   │ IPC Router │   │ Event Bus  │   │  Settings   │   │
 │   │  Manager  │   │ (Specta)   │   │  (typed)   │   │   Store     │   │
 │   └─────┬─────┘   └─────┬──────┘   └─────┬──────┘   └──────┬──────┘   │
 │         │               │                │                 │          │
 │         ▼               ▼                ▼                 ▼          │
 │   ┌───────────────────────── Permission Broker ──────────────────┐    │
 │   │          ◉ every capability use passes through here          │    │
 │   └───────────────────────────────┬──────────────────────────────┘    │
 │                                   │                                   │
 │   ┌─────────────────────── Plugin Registry ──────────────────────┐    │
 │   │   WASM Runtime (wasmtime component model)   │  Native ABI    │    │
 │   └──────────────────────────────────────────────────────────────┘    │
 └─────▲──────────────▲──────────────▲──────────────▲──────────────▲─────┘
       │              │              │              │              │
  ┌────┴───┐    ┌─────┴────┐   ┌─────┴───┐    ┌─────┴───┐    ┌─────┴───┐
  │ plugin │    │ plugin   │   │ plugin  │    │ plugin  │    │ plugin  │
  │  chat  │    │ terminal │   │   fs    │    │  agent  │    │  ...    │
  │ (wasm) │    │ (native) │   │ (wasm)  │    │ (wasm)  │    │         │
  └────────┘    └──────────┘   └─────────┘    └─────────┘    └─────────┘
```

## 2. Crate Layout (summary)

Full tree with per-file purposes lives in [`project_structure.md`](./project_structure.md). Summary:

```
rustacle/
├── crates/
│   ├── rustacle-kernel          # lifecycle, registry, event bus, permission broker
│   ├── rustacle-ipc             # Specta types, command/event contracts
│   ├── rustacle-plugin-api      # host-side RustacleModule trait (adapter over WIT)
│   ├── rustacle-plugin-wit      # *.wit component interfaces (contract surface)
│   ├── rustacle-wasm-host       # wasmtime host: loader, linker, capability wiring
│   ├── rustacle-settings        # typed settings store (SQLite-backed)
│   ├── rustacle-llm             # LlmProvider trait + provider registry
│   ├── rustacle-llm-openai      # OpenAI-compatible provider
│   ├── rustacle-llm-anthropic   # Anthropic provider
│   ├── rustacle-llm-local       # Ollama/LM-Studio/llama.cpp-server provider
│   └── rustacle-app             # Tauri binary; thin shell wiring UI ↔ kernel
└── plugins/
    ├── fs                       # wasm  — read/list/stat/search, scoped
    ├── terminal                 # native (portable-pty, whitelisted)
    ├── chat                     # wasm  — user chat turns, history
    ├── agent                    # wasm  — thinking loop, calls host LlmProvider
    ├── memory                   # wasm  — long-term memory store
    └── skills                   # wasm  — user-defined tool loader
```

**Why split `rustacle-plugin-api` from `rustacle-plugin-wit` from `rustacle-kernel`?** The WIT file is the **contract** (semver-stable). The host trait is the **adapter** the kernel uses to talk to a loaded plugin. The kernel itself can evolve without breaking plugins, and plugins can evolve without knowing about kernel internals.

## 3. IPC Layer

### 3.1 Rules

- **Tauri v2 commands** = request/response. Used for everything with a return value.
- **Tauri events** = server-push streams (reasoning steps, PTY bytes, progress, cost samples).
- **`specta + tauri-specta`** generates `bindings.ts` from Rust types. **A hand-written TS IPC type is a bug.** CI fails if `bindings.ts` drifts from the Rust source.
- `rustacle-ipc` is the **only** crate allowed to depend on Tauri's API version. v3 migration is isolated here.

### 3.2 Example command (pseudocode)

```rust
// crates/rustacle-ipc/src/commands/plugins.rs
#[derive(Serialize, Deserialize, specta::Type)]
pub struct ListPluginsResponse {
    pub plugins: Vec<PluginSummary>,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct PluginSummary {
    pub id: String,
    pub version: String,
    pub state: PluginState,
    pub granted_capabilities: Vec<Capability>,
    pub pending_capabilities: Vec<Capability>,
}

#[tauri::command]
#[specta::specta]
pub async fn list_plugins(state: tauri::State<'_, AppState>)
    -> Result<ListPluginsResponse, RustacleError>
{
    Ok(ListPluginsResponse {
        plugins: state.kernel.registry.snapshot().await,
    })
}
```

Generated TS side:

```ts
// bindings.ts (generated, DO NOT EDIT)
export async function listPlugins(): Promise<ListPluginsResponse> { ... }
export type PluginSummary = { id: string; version: string; state: PluginState; ... };
```

### 3.3 Event example

```rust
// One-way stream from kernel to UI. Typed topic.
#[derive(Serialize, specta::Type, Clone)]
pub struct ReasoningStepEvent {
    pub turn_id: TurnId,
    pub step: ReasoningStep,
}

// Emission: kernel bridges from the event bus to Tauri.
app_handle.emit("agent:reasoning", ReasoningStepEvent { turn_id, step })?;
```

TS subscribes via the generated `listen("agent:reasoning", …)` helper.

## 4. The Plugin System

### 4.1 WASM-first

Plugins are **WebAssembly Component Model** artifacts loaded by `wasmtime`. The component model buys us:

- **Capability-based sandboxing.** The plugin sees **only** host functions it was linked against. No ambient authority, no filesystem, no net.
- **Language neutrality.** Plugins in Rust today; Zig/Go/Python-via-componentize-py later.
- **Hot-swap.** Unload an instance, load a new version, no process restart.
- **Portability.** Same `.wasm` on Windows, macOS, Linux.

**Native fallback** exists for capabilities WASI cannot yet express. Today the only native plugin is `plugins/terminal` (PTY spawn). Each native plugin carries an explicit migration note pointing at the WASI proposal that would unblock it (PTY → WASI Preview 3 `wasi:io/streams` + a `wasi:process/spawn` world).

Native plugins are **compiled into the host binary**, whitelisted by a feature flag, and signed the same as WASM plugins for update integrity.

### 4.2 Contract surface: WIT

The plugin contract is a **WIT** (WebAssembly Interface Types) file in `crates/rustacle-plugin-wit/wit/rustacle.wit`:

```wit
package rustacle:plugin@0.1.0;

interface types {
    record module-manifest {
        id: string,
        version: string,
        capabilities: list<capability>,
        subscriptions: list<string>,
        ui-contributions: ui-contributions,
    }

    variant capability {
        fs(fs-scope),
        net(net-scope),
        pty,
        secret(string),
        llm-provider,
    }

    record fs-scope { paths: list<string>, mode: fs-mode }
    enum fs-mode { read, read-write }
    record net-scope { hosts: list<string> }

    record ui-contributions {
        panels: list<panel-desc>,
        palette-entries: list<palette-entry>,
        settings-schema: string, // JSON-schema blob, rendered by the Settings UI
    }

    variant module-error {
        denied(string),
        invalid-input(string),
        internal(string),
    }
}

/// Exported by the plugin.
interface module {
    use types.{module-manifest, module-error};

    manifest: func() -> module-manifest;
    init:     func() -> result<_, module-error>;
    on-event: func(topic: string, payload: list<u8>) -> result<_, module-error>;
    shutdown: func() -> result<_, module-error>;

    /// RPC: the kernel invokes plugin-declared commands by name with a typed payload.
    call: func(command: string, payload: list<u8>) -> result<list<u8>, module-error>;
}

/// Imported by the plugin (host functions).
interface host {
    use types.{capability, module-error};

    /// Event bus.
    publish: func(topic: string, payload: list<u8>);

    /// Permission-gated I/O.
    fs-read:    func(path: string) -> result<list<u8>, module-error>;
    fs-write:   func(path: string, data: list<u8>) -> result<_, module-error>;
    net-fetch:  func(url: string, body: list<u8>) -> result<list<u8>, module-error>;
    secret-get: func(key: string) -> result<string, module-error>;

    /// LLM router (plugins never talk to providers directly).
    llm-stream: func(req: list<u8>) -> result<u64, module-error>; // stream-id
    llm-poll:   func(stream-id: u64) -> result<llm-chunk, module-error>;

    /// Structured logging.
    log: func(level: string, message: string, fields: list<u8>);
}

world plugin {
    import host;
    export module;
}
```

This file is the **single source of truth** for the plugin contract. `wit-bindgen` generates Rust bindings on both sides (host and guest). A change here is a breaking change; bump the package version and update every plugin.

### 4.3 Host-side adapter: `RustacleModule` trait

On the host, the kernel bridges each loaded WASM component into a uniform Rust trait. This trait is not implemented by plugin authors — it's what the **kernel** uses to drive a plugin.

```rust
// crates/rustacle-plugin-api/src/lib.rs

/// Host-side handle to a loaded plugin (WASM or native).
/// Plugin authors DO NOT implement this — they export the WIT `module` interface,
/// and `rustacle-wasm-host` adapts it into this trait.
#[async_trait::async_trait]
pub trait RustacleModule: Send + Sync {
    fn id(&self) -> &str;
    fn manifest(&self) -> &ModuleManifest;

    async fn init(&mut self) -> Result<(), ModuleError>;
    async fn on_event(&mut self, topic: &str, payload: Bytes) -> Result<(), ModuleError>;
    async fn call(&mut self, command: &str, payload: Bytes) -> Result<Bytes, ModuleError>;
    async fn shutdown(&mut self) -> Result<(), ModuleError>;
}

#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
pub struct ModuleManifest {
    pub id: String,
    pub version: semver::Version,
    pub capabilities: Vec<Capability>,
    pub subscriptions: Vec<TopicFilter>,
    pub ui_contributions: UiContributions,
}

#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
pub enum Capability {
    Fs { scope: PathScope, mode: FsMode },
    Net { allow_hosts: Vec<HostPattern> },
    Pty,                  // native-only
    Secret { key: String },
    LlmProvider,          // may call host llm-stream/llm-poll
}

#[derive(thiserror::Error, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", content = "data")]
pub enum ModuleError {
    #[error("permission denied: {capability}")]
    Denied { capability: String },
    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },
    #[error("wasm trap: {0}")]
    Trap(String),
    #[error("internal: {0}")]
    Internal(String),
}
```

### 4.4 Lifecycle and state

```
discover → verify_signature → negotiate_capabilities → init
                                                         │
                                                         ▼
                                                   ┌──running──┐
                                                   │           │
                                                   ▼           ▼
                                               suspended   hot-swap
                                                   │           │
                                                   └──shutdown ◀┘
```

- **`discover`**: scan the plugins dir + the host's whitelisted native list.
- **`verify_signature`**: Ed25519 verify against a trusted key list. Failure → plugin refused.
- **`negotiate_capabilities`**: the kernel presents each requested capability to the user through the Settings UI. If the user denies a mandatory capability, the plugin fails to start with a visible reason.
- **`init`**: `module.init()` runs; the plugin subscribes to topics.
- **`hot-swap`**: see §4.5.
- **`suspend`**: all inbound commands reply `Unavailable`; subscriptions are buffered under the `CoalesceLatest` policy.

### 4.5 Hot-swap & state migration

The registry tracks `(plugin_id, version, instance_handle)`. On update:

1. Load new version in a fresh wasmtime Store.
2. Call `old.export_state()` → opaque `Bytes` (plugin-defined schema).
3. Call `new.import_state(bytes)` — if it fails, abort swap and keep old instance.
4. Drain in-flight commands on old (bounded timeout; after that, cancel).
5. Atomic registry entry swap under a single `RwLock` write.
6. Drop old store.
7. Event-bus subscriptions are re-bound by topic name; in-flight events delivered to the new instance.

**State migration policy** is per-plugin, declared in the manifest:

- `Transient` — state is discarded on swap (default for leaf plugins like `fs`).
- `Serialized` — plugin implements `export_state`/`import_state`; the host enforces a max size (1 MiB default).
- `ExternalStore` — state lives outside the plugin (in SQLite); swap is trivial. Used by `memory` and `chat`.

### 4.6 Event Bus

```rust
// crates/rustacle-kernel/src/bus.rs

/// A typed topic; backpressure policy is declared at publish site.
pub struct Topic<T> {
    name: &'static str,
    policy: BackpressurePolicy,
    _marker: PhantomData<T>,
}

pub enum BackpressurePolicy {
    /// Publisher awaits until subscriber has room. Use for must-not-lose events
    /// like agent.reasoning, agent.toolcall, permission.ask.
    BlockPublisher,
    /// Drop oldest buffered item. Use for idempotent progress updates.
    DropOldest,
    /// Keep only the latest item per subscriber. Use for state-summary topics
    /// like agent.cost, terminal.cwd.
    CoalesceLatest,
}

pub struct Bus {
    topics: DashMap<&'static str, BoxedTopicHandle>,
}

impl Bus {
    pub fn publish<T: Send + Clone + 'static>(
        &self,
        topic: &Topic<T>,
        event: T,
    ) -> Result<(), BusError> { ... }

    pub fn subscribe<T: Send + Clone + 'static>(
        &self,
        topic: &Topic<T>,
        plugin_id: &str,
    ) -> mpsc::Receiver<T> { ... }
}
```

**Topic registry** (stable list — see `project_structure.md` for live entries):

| Topic | Payload | Policy | Publisher | Typical subscriber |
|---|---|---|---|---|
| `agent.reasoning` | `ReasoningStep` | `BlockPublisher` | `plugins/agent` | UI, `memory` |
| `agent.cost` | `CostSample` | `CoalesceLatest` | `plugins/agent` | UI, `settings` |
| `permission.ask` | `PermissionAsk` | `BlockPublisher` | kernel | UI |
| `terminal.output` | `TerminalChunk` | `DropOldest` | `plugins/terminal` | UI |
| `terminal.cwd` | `CwdChange` | `CoalesceLatest` | `plugins/terminal` | `plugins/agent` |
| `fs.selected` | `SelectedFiles` | `CoalesceLatest` | UI | `plugins/agent` |

### 4.7 Permission Broker

A single function on the host:

```rust
pub struct PermissionBroker {
    grants: DashMap<(PluginId, CapabilityKey), Grant>,
    settings: Arc<SettingsStore>,
    ask_tx:   mpsc::Sender<PermissionAsk>,
}

impl PermissionBroker {
    pub async fn check(
        &self,
        plugin: &PluginId,
        cap: &Capability,
    ) -> Result<(), ModuleError> {
        let key = CapabilityKey::from(cap);
        if let Some(g) = self.grants.get(&(plugin.clone(), key.clone())) {
            return g.decision.as_result(cap);
        }
        // Not cached → ask the user via UI.
        let (reply_tx, reply_rx) = oneshot::channel();
        self.ask_tx.send(PermissionAsk { plugin: plugin.clone(), cap: cap.clone(), reply_tx }).await?;
        let decision = reply_rx.await?;
        self.grants.insert((plugin.clone(), key), Grant { decision: decision.clone(), ts: now() });
        decision.as_result(cap)
    }

    /// Called when the user edits permissions in Settings UI.
    pub fn invalidate(&self, plugin: &PluginId, key: &CapabilityKey) {
        self.grants.remove(&(plugin.clone(), key.clone()));
    }
}
```

**Cache key discipline for parameterized capabilities:**
- `Fs { scope: /home/k/projects, mode: Read }` uses `CapabilityKey::Fs(canonicalize(/home/k/projects), Read)`. Checks canonicalize the **requested** path and verify prefix match against a granted scope; the cache holds **scope grants**, not per-path decisions.
- `Net { hosts: ["api.openai.com"] }` cache holds host patterns; per-request check is `host_pattern_match`.
- `Secret { key: "openai_api_key" }` cache holds key-exact grants.

Every cache hit is still logged (at `trace` level) so audit trails survive.

## 5. Kernel State Model

```rust
// crates/rustacle-kernel/src/state.rs

pub struct AppState {
    pub kernel:     Arc<Kernel>,
    pub settings:   Arc<SettingsStore>,
    pub bus:        Arc<Bus>,
    pub llm:        Arc<LlmRegistry>,
    pub permission: Arc<PermissionBroker>,
}

pub struct Kernel {
    pub registry: RwLock<PluginRegistry>,
    pub tasks:    JoinSet<()>,     // owns every long-running task
    pub shutdown: CancellationToken,
}
```

Rules:
- **No `static mut`.** No `lazy_static` for mutable data. No `OnceCell` for anything the user can reconfigure at runtime.
- **No cross-plugin shared memory.** Plugins communicate through the bus or through kernel-mediated commands.
- **Every `tokio::spawn` is owned by a `JoinSet`** so shutdown can await completion.

## 6. Threat Model (summary)

Full STRIDE analysis lives in `knowledge_base.md` §4. Summary of attack surfaces and mitigations:

| Surface | Threat | Mitigation |
|---|---|---|
| Malicious plugin `.wasm` | Escape sandbox, exfiltrate data | wasmtime fuel metering + memory limits + narrow host imports + Ed25519 signatures + permission broker |
| TOCTOU on FS plugin | Write outside scope via symlink | Canonicalize before scope check, re-check after open where possible |
| Cached permission after user revokes | Plugin keeps reading after revocation | Broker `invalidate()` called on every Settings change |
| LLM prompt injection | Agent exfiltrates secrets via tool call | Secrets never appear in prompts; `LlmProvider` host function strips known secret values from request payloads as defense-in-depth |
| Settings import from untrusted source | Malicious capability grants | Import UI shows a diff; grants require user click per plugin |
| Log lines containing secrets | Disk leak | `SecretString` redacts on `Debug`; `tracing` fields go through a redactor |

## 7. Open Questions

Tracked as ADRs:
- ADR-0001: UI framework (Solid vs React) — see [`adr/0001-ui-framework.md`](./adr/0001-ui-framework.md).
- ADR-0002: Settings import format (JSON vs CBOR vs TOML with typed schema) — pre-Sprint-5.
- ADR-0003: Plugin signing key distribution — pre-Sprint-8.

---
*Related: [README](./README.md) · [project_structure](./project_structure.md) · [agent_reasoning](./agent_reasoning.md) · [tech_stack_2026](./tech_stack_2026.md) · [knowledge_base](./knowledge_base.md) · [glossary](./glossary.md)*
