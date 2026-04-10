# Project Structure

> The authoritative Cargo workspace layout for Rustacle. Every crate, every top-level `src/` file, and its purpose. Keep this file in sync with reality — a drift between this doc and the tree is a PR blocker.

## Top-level tree

```
rustacle/
├── Cargo.toml                    # [workspace] manifest — lists all crates, shared deps
├── Cargo.lock
├── rust-toolchain.toml           # pinned stable channel, 2024 edition
├── .cargo/config.toml            # target dir, linker, common rustflags
├── README.md                     # project README (user-facing)
├── for_dev/                      # THIS directory — architectural canon
├── crates/                       # host-side Rust crates
├── plugins/                      # plugin crates (wasm + whitelisted native)
├── ui/                           # Tauri webview frontend (Solid or React — see ADR-0001)
├── assets/                       # icons, themes, default skills
├── migrations/                   # sqlx migrations for the settings/history DB
├── keys/                         # trusted plugin signing public keys
├── tests/                        # workspace-level integration & e2e tests
└── scripts/                      # dev scripts (regen bindings, build plugins, …)
```

## Workspace crates (`crates/`)

### `rustacle-kernel` — the micro-kernel

```
crates/rustacle-kernel/
├── Cargo.toml
└── src/
    ├── lib.rs                    # pub-use barrel
    ├── kernel.rs                 # Kernel { registry, tasks, shutdown }
    ├── state.rs                  # AppState
    ├── lifecycle.rs              # discover / verify / init / shutdown
    ├── registry.rs               # PluginRegistry, hot-swap logic
    ├── bus/
    │   ├── mod.rs                # Bus, Topic<T>
    │   ├── policy.rs             # BackpressurePolicy
    │   └── topics.rs             # static topic registry (well-known topics)
    ├── permission/
    │   ├── mod.rs                # PermissionBroker
    │   ├── key.rs                # CapabilityKey canonicalization
    │   └── ask.rs                # PermissionAsk event flow
    ├── ipc/
    │   ├── mod.rs                # Tauri command registration glue
    │   └── router.rs             # command dispatch to registered plugins
    └── errors.rs                 # KernelError (internal), maps to ipc RustacleError
```

### `rustacle-ipc` — the typed bridge

```
crates/rustacle-ipc/
├── Cargo.toml
└── src/
    ├── lib.rs                    # export_bindings() for build.rs
    ├── errors.rs                 # RustacleError (tagged enum, #[serde(tag="kind")])
    ├── commands/
    │   ├── mod.rs
    │   ├── plugins.rs            # list_plugins, grant_capability, hot_swap
    │   ├── settings.rs           # get_settings, set_setting, import/export
    │   ├── agent.rs              # start_turn, cancel_turn, replay_turn
    │   ├── terminal.rs           # open_tab, close_tab, write_pty, resize
    │   └── fs.rs                 # fs_select, fs_unselect, fs_open_preview
    └── events/
        ├── mod.rs
        ├── agent.rs              # ReasoningStepEvent, CostSampleEvent
        ├── terminal.rs           # TerminalChunkEvent, CwdChangeEvent
        └── permission.rs         # PermissionAskEvent
```

### `rustacle-plugin-wit` — the WIT contract

```
crates/rustacle-plugin-wit/
├── Cargo.toml
└── wit/
    └── rustacle.wit              # the one file — see architecture.md §4.2
```

### `rustacle-plugin-api` — host-side adapter trait

```
crates/rustacle-plugin-api/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── module.rs                 # RustacleModule async trait
    ├── manifest.rs               # ModuleManifest, UiContributions
    ├── capability.rs             # Capability, PathScope, HostPattern
    └── errors.rs                 # ModuleError
```

### `rustacle-wasm-host` — wasmtime integration

```
crates/rustacle-wasm-host/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── host.rs                   # Store<HostState> setup, fuel/memory limits
    ├── linker.rs                 # wit-bindgen host imports (fs_read, net_fetch, …)
    ├── loader.rs                 # .wasm loader + signature verification
    ├── adapter.rs                # impl RustacleModule for WasmtimeInstance
    ├── llm_bridge.rs             # llm-stream / llm-poll host fns
    └── state_migration.rs        # export_state/import_state policy
```

### `rustacle-settings` — the Zero-JSON backing store

```
crates/rustacle-settings/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── store.rs                  # SettingsStore (SQLite)
    ├── schema.rs                 # typed keys, versioned
    ├── import_export.rs          # diff, apply, export (excludes secrets)
    ├── secrets.rs                # keyring wrapper, SecretString
    └── migrations.rs             # re-export sqlx::migrate!
```

### `rustacle-llm` — provider abstraction

```
crates/rustacle-llm/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── provider.rs               # LlmProvider async trait
    ├── registry.rs               # LlmRegistry, profile → provider routing
    ├── types.rs                  # ChatRequest, ChatDelta, ToolSchema, TokenCost
    └── router.rs                 # bridges from plugin `llm-stream` host fn to provider
```

### `rustacle-llm-openai` / `rustacle-llm-anthropic` / `rustacle-llm-local`

```
crates/rustacle-llm-openai/
├── Cargo.toml
└── src/
    ├── lib.rs                    # impl LlmProvider (OpenAI dialect)
    └── streaming.rs              # SSE parsing, tool-use translation

crates/rustacle-llm-anthropic/
├── Cargo.toml
└── src/
    ├── lib.rs                    # impl LlmProvider (Anthropic tool-use dialect)
    └── streaming.rs

crates/rustacle-llm-local/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Ollama / LM-Studio / llama.cpp — all OpenAI-compatible
    └── discovery.rs              # auto-detect local servers at startup
```

### `rustacle-app` — the Tauri binary

```
crates/rustacle-app/
├── Cargo.toml
├── tauri.conf.json               # Tauri config (windows, bundler, updater)
├── icons/
├── build.rs                      # regenerates bindings.ts via tauri-specta
└── src/
    ├── main.rs                   # tauri::Builder, wires AppState, registers commands
    ├── setup.rs                  # on-startup: load plugins, run migrations, init bus
    ├── menu.rs                   # native menu bar (mac/linux)
    └── updater.rs                # Tauri updater glue
```

## Plugin crates (`plugins/`)

### `plugins/fs` (wasm)

```
plugins/fs/
├── Cargo.toml                    # crate-type = ["cdylib"], cargo-component target
└── src/
    ├── lib.rs                    # wit-bindgen export of `module` interface
    ├── commands.rs               # read_file, list_dir, stat, search
    ├── selection.rs              # selected_files set, publishes fs.selected
    └── scopes.rs                 # scope checks client-side (defense in depth)
```

### `plugins/terminal` (native, whitelisted)

```
plugins/terminal/
├── Cargo.toml
└── src/
    ├── lib.rs                    # impl RustacleModule directly (no wasm)
    ├── pty.rs                    # portable-pty spawn/resize/write
    ├── tabs.rs                   # tab state, per-tab context
    ├── splits.rs                 # recursive split tree
    └── parser.rs                 # vt100 wrap, outputs TerminalChunk events
```

### `plugins/chat` (wasm)

```
plugins/chat/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── history.rs                # conversation history (persisted via external store)
    └── commands.rs               # post_user_turn, rewind, fork
```

### `plugins/agent` (wasm) — the brain

```
plugins/agent/
├── Cargo.toml
└── src/
    ├── lib.rs                    # wit-bindgen export
    ├── harness/
    │   ├── mod.rs                # Harness struct, run_turn()
    │   ├── loop.rs               # the Thinking loop
    │   ├── dispatch.rs           # ToolDispatchTable
    │   ├── streaming.rs          # partial-thought flushing
    │   └── cancel.rs             # CancellationToken wiring
    ├── prompt/
    │   ├── mod.rs                # assemble_prompt()
    │   ├── layers.rs             # SYSTEM_BASE, env_context, memory, history
    │   ├── tools.rs              # render ToolSchema list
    │   └── golden_tests.rs       # insta snapshots
    ├── tools/
    │   ├── mod.rs                # Tool trait (plugin-internal)
    │   ├── registry.rs
    │   ├── bash.rs               # example tool — delegates to terminal plugin
    │   ├── fs_read.rs
    │   ├── fs_edit.rs
    │   ├── fs_write.rs
    │   ├── grep.rs
    │   ├── glob.rs
    │   └── sub_agent.rs          # spawn a child harness
    └── errors.rs
```

### `plugins/memory` (wasm)

```
plugins/memory/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── store.rs                  # SQLite-backed (via host fn `kv-*` in v0.2)
    ├── scoring.rs                # simple BM25 + recency decay
    └── commands.rs               # remember, forget, recall
```

### `plugins/skills` (wasm)

```
plugins/skills/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── loader.rs                 # discover user skills from the skills dir
    └── invoke.rs                 # call a skill as a tool
```

## Frontend (`ui/`)

Framework decided in ADR-0001. Tree shape illustrates the target for Solid; React would differ in hook files.

```
ui/
├── package.json
├── vite.config.ts
├── index.html
├── tsconfig.json
├── bindings.ts                   # GENERATED from Rust via tauri-specta — DO NOT EDIT
└── src/
    ├── main.tsx                  # root, mounts <App/>
    ├── App.tsx
    ├── ipc/
    │   ├── commands.ts           # thin wrappers around bindings.ts
    │   └── events.ts             # topic → signal/store adapters
    ├── components/
    │   ├── terminal/
    │   │   ├── Tab.tsx           # XTerm.js host
    │   │   ├── TabBar.tsx
    │   │   ├── SplitTree.tsx
    │   │   └── useTerminal.ts
    │   ├── agent/
    │   │   ├── AgentPanel.tsx
    │   │   ├── ReasoningCard.tsx
    │   │   ├── ThoughtCard.tsx
    │   │   ├── ToolCallCard.tsx
    │   │   ├── PermissionCard.tsx
    │   │   └── CostBadge.tsx
    │   ├── settings/
    │   │   ├── SettingsPage.tsx
    │   │   ├── ModelProfiles.tsx
    │   │   ├── Permissions.tsx
    │   │   ├── Keybindings.tsx
    │   │   ├── Themes.tsx
    │   │   └── ImportExport.tsx
    │   ├── palette/
    │   │   └── CommandPalette.tsx
    │   └── common/
    ├── state/                    # Solid stores or Zustand slices
    ├── themes/                   # CSS custom-property bundles
    └── i18n/
```

## Assets & data dirs

```
assets/
├── icons/
├── themes/                       # default theme bundles (JSON schema → UI import)
└── skills/                       # stock skills shipped in the bundle

migrations/
├── 0001_init.sql
├── 0002_settings.sql
└── 0003_reasoning.sql

keys/
└── trusted_plugin_keys.toml      # list of Ed25519 pubkeys, names, expiry
```

## Test layout

```
tests/
├── kernel/                       # integration tests (one-plugin harness)
├── ipc/                          # specta bindings regression (insta)
├── agent/                        # prompt golden tests (insta)
├── plugins/                      # per-plugin contract tests
└── e2e/                          # Playwright via tauri-driver
```

## Scripts

```
scripts/
├── regen-bindings.sh             # cargo run -p rustacle-app -- --regen-bindings
├── build-plugins.sh              # builds every plugins/*/ with cargo-component
├── sign-plugin.sh                # Ed25519 sign a .wasm
└── check-wit.sh                  # wit-parser validate the .wit file
```

---
*Related: [README](./README.md) · [architecture](./architecture.md) · [tech_stack_2026](./tech_stack_2026.md)*
