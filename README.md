<p align="center">
  <h1 align="center">Rustacle</h1>
  <p align="center">
    <strong>A local-first agentic UI controller</strong> — see what the AI thinks, control what it can do, extend it with plugins.
  </p>
  <p align="center">
    Built with Rust + Tauri v2 · WASM plugin sandbox · Visible agent reasoning
  </p>
  <p align="center">
    <a href="https://github.com/robotoss/Rustacle/issues">Report a Bug</a> ·
    <a href="https://github.com/robotoss/Rustacle/issues">Request a Feature</a> ·
    <a href="#contributing">Contribute</a>
  </p>
</p>

---

> **Rustacle is not a terminal emulator.** It is a **desktop agent controller** — a native UI where you interact with AI agents that can use tools, access files, and run commands on your behalf. Every action is visible, every capability is permissioned, and every feature is a hot-swappable plugin.

## Why Rustacle?

Today's AI tools hide their reasoning, scatter config across JSON files, and give you no control over what the agent can access. Rustacle is different:

- **See everything** — every Thought, ToolCall, and Answer streams as a typed event card in real time. No hidden reasoning.
- **Control everything** — a Permission Broker gates every capability. You decide what the agent can touch, from the UI.
- **Configure without files** — every setting has a UI control. You never edit a config file.
- **Run local or cloud** — same UX for Ollama, LM Studio, OpenAI, or Anthropic. Your data stays on your machine.
- **Extend with plugins** — write a tool in Rust (or JavaScript), compile to WASM, drop it in. Done.

## How It Works

```
┌─────────────────────────────────────────────────┐
│                   Rustacle UI                   │
│         React 19 · Tailwind · Zustand           │
├─────────────────────────────────────────────────┤
│              Type-safe IPC (specta)              │
├─────────────────────────────────────────────────┤
│                  Micro-kernel                   │
│     Lifecycle · Event Bus · Permission Broker   │
├──────────┬──────────┬──────────┬────────────────┤
│  Agent   │    FS    │   Chat   │   Your Plugin  │
│  Plugin  │  Plugin  │  Plugin  │    (WASM)      │
│  (WASM)  │  (WASM)  │  (WASM)  │                │
└──────────┴──────────┴──────────┴────────────────┘
```

The **micro-kernel** handles only lifecycle, IPC, permissions, and the event bus. Everything else — chat, file access, agent reasoning, memory — is a **sandboxed WASM plugin** that declares its capabilities and gets permission-checked at runtime.

## Features

| Feature | Description |
|---------|-------------|
| **Visible Reasoning** | Agent thoughts, tool calls, and answers stream as typed event cards |
| **Permission Broker** | Every plugin capability is gated, cached, and user-controllable |
| **WASM Plugin Sandbox** | Ed25519-signed, capability-gated, hot-swappable modules |
| **Zero-JSON Config** | Every setting lives in a UI control, never a config file |
| **Type-safe IPC** | Auto-generated TypeScript bindings from Rust types via tauri-specta |
| **Local + Cloud Models** | One trait, same UX for Ollama, LM Studio, OpenAI, Anthropic |
| **Language-neutral Plugins** | Write plugins in Rust, JavaScript, or any WASM-targeting language |
| **Cross-platform** | Windows, macOS, Linux via Tauri v2 |
| **MCP Support** | External tools and resources gated by the same permission broker |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Tauri v2, tokio, tracing |
| Frontend | React 19, Vite, Tailwind CSS v4, TypeScript |
| IPC | tauri-specta (typed, auto-generated `bindings.ts`) |
| Plugins | WebAssembly Component Model (wasmtime, WIT, cargo-component) |
| Signing | Ed25519 (ed25519-dalek) |
| State | Zustand (UI), SQLite (settings/history) |
| Testing | nextest, insta, proptest, Playwright |

## Quick Start

### Prerequisites

- **Rust** stable (1.85+)
- **Node.js** 22+
- **cargo-component**: `cargo install cargo-component`
- **wasm32-wasip1 target**: `rustup target add wasm32-wasip1`
- Platform dependencies:
  - **Windows**: Visual Studio Build Tools (MSVC)
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`

### Install and Run

```bash
# Clone
git clone https://github.com/robotoss/Rustacle.git
cd rustacle

# Install frontend dependencies
cd ui && npm install && cd ..

# Build frontend (required before first run)
cd ui && npm run build && cd ..

# Run
cargo run -p rustacle-app
```

<details>
<summary><strong>Windows (PowerShell)</strong></summary>

```powershell
git clone https://github.com/robotoss/Rustacle.git
cd rustacle
cd ui; npm install; cd ..
cd ui; npm run build; cd ..
cargo run -p rustacle-app
```

</details>

## Project Structure

```
rustacle/
├── crates/                       # Host-side Rust crates (workspace members)
│   ├── rustacle-kernel/          # Micro-kernel: lifecycle, bus, permissions
│   ├── rustacle-ipc/             # Typed IPC commands, events, errors
│   ├── rustacle-plugin-api/      # RustacleModule trait, Capability, Manifest
│   ├── rustacle-plugin-wit/      # WIT contract (rustacle:plugin@0.1.0)
│   ├── rustacle-wasm-host/       # Wasmtime: loader, linker, fuel/memory limits
│   ├── rustacle-settings/        # Zero-JSON settings store
│   ├── rustacle-llm/             # LLM provider abstraction
│   ├── rustacle-llm-openai/      # OpenAI-compatible provider
│   ├── rustacle-llm-anthropic/   # Anthropic provider
│   ├── rustacle-llm-local/       # Ollama / LM Studio provider
│   ├── rustacle-agent/           # Agent prompt assembly, harness loop, tool dispatch
│   └── rustacle-app/             # Tauri v2 binary + IPC commands
├── plugins/                      # Plugin crates (WASM + native)
│   ├── fs/                       # File system plugin (WASM)
│   ├── terminal/                 # PTY terminal (native — WASI can't do PTY yet)
│   ├── chat/                     # Chat history (WASM)
│   ├── agent/                    # Agent reasoning loop (WASM)
│   ├── memory/                   # Long-term memory (WASM)
│   └── skills/                   # User-defined tools (WASM)
├── ui/                           # React 19 + Vite + Tailwind CSS v4
├── keys/                         # Trusted Ed25519 public keys
├── scripts/                      # Dev scripts
└── for_dev/                      # Architectural docs (for contributors)
```

## Plugin System

Rustacle uses the **WASM Component Model** for plugins. Every feature (chat, agent, FS, memory) is a `.wasm` module loaded at runtime.

**How it works:**
1. Plugins implement a [WIT interface](crates/rustacle-plugin-wit/wit/rustacle.wit)
2. The host provides sandboxed APIs: `fs-read`, `fs-write`, `net-fetch`, `secret-get`, `llm-stream`, `publish`, `log`
3. Each plugin declares required capabilities in its manifest
4. The **Permission Broker** checks every capability use against user grants
5. All `.wasm` files must be **Ed25519-signed** — unsigned plugins are refused

### Building a Plugin

```bash
# Rust plugin
cd plugins/fs && cargo component build

# JavaScript plugin (any language that targets WASM Component Model works)
jco componentize plugins/hello-js/plugin.js \
  --wit crates/rustacle-plugin-wit/wit/ \
  --world-name plugin \
  --out plugins/hello-js/hello-js.wasm

# Sign before loading
bash scripts/sign-plugin.sh target/wasm32-wasip1/debug/rustacle_plugin_fs.wasm
```

| Plugin | Language | Commands |
|--------|----------|----------|
| `plugins/fs` | Rust | read_file, list_dir, stat |
| `plugins/hello-js` | JavaScript | greet, ping, info |

### Demo Plugin

A built-in `DemoPlugin` proves the full UI → IPC → Kernel → Plugin pipeline. Run the app and click **"Ping From Plugin"** or use the echo input.

## Development

```bash
# Run all checks
bash scripts/check-local.sh

# Or individually:
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo nextest run --workspace

# Regenerate TypeScript bindings after changing Rust IPC types
bash scripts/regen-bindings.sh
# or: cargo run -p rustacle-app --bin export_bindings

# Build WASM plugins
bash scripts/build-plugins.sh

# Run with debug logging
RUSTACLE_LOG=debug cargo run -p rustacle-app
```

<details>
<summary><strong>Windows (PowerShell)</strong></summary>

```powershell
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo nextest run --workspace
cargo run -p rustacle-app --bin export_bindings
cd plugins/fs; cargo component build; cd ../..
$env:RUSTACLE_LOG="debug"; cargo run -p rustacle-app
```

</details>

## Testing

| Layer | Tool | What it covers |
|-------|------|----------------|
| Unit | `cargo test` | Pure functions, permission logic, path scoping |
| Integration | nextest | Kernel + single plugin harness |
| Golden | insta | Prompt assembly snapshots |
| Property | proptest | Path canonicalization, backpressure |
| E2E | Playwright | Full Tauri app UI flows |

**Current tests (48):**

| Crate | Test | What it verifies |
|-------|------|-----------------|
| `rustacle-kernel` | `kernel_start_stop` | Kernel lifecycle (start, stop, cancellation) |
| `rustacle-kernel` | `registry_register_and_call` | Plugin registration + command routing |
| `rustacle-kernel` | `registry_call_unknown_plugin` | Error on unknown plugin |
| `rustacle-kernel` | `permission_allow_is_cached` | Grant caching in broker |
| `rustacle-kernel` | `permission_deny_is_not_cached` | Deny not cached (retryable) |
| `rustacle-kernel` | `invalidate_removes_grant` | Cache invalidation works |
| `rustacle-kernel` | `broadcast_publish_subscribe` | Event bus broadcast pub/sub |
| `rustacle-kernel` | `watch_coalesce_latest` | Event bus coalesce-latest policy |
| `rustacle-kernel` | `publish_to_unknown_topic_fails` | Bus error on unknown topic |
| `rustacle-kernel` | `register_all_terminal_topics` | Well-known topic registration |
| `rustacle-plugin-api` | `path_scope_contains` | FS path scope segment matching |
| `rustacle-plugin-api` | `host_pattern_exact` | Exact host matching |
| `rustacle-plugin-api` | `host_pattern_wildcard` | Wildcard `*.example.com` matching |
| `rustacle-plugin-terminal` | `detect_shell_returns_something` | Shell auto-detection |
| `rustacle-plugin-terminal` | `pty_spawn_and_alive` | PTY spawn and child alive |
| `rustacle-plugin-terminal` | `pty_write_and_read` | PTY write/read round-trip |
| `rustacle-wasm-host` | `js_plugin_component_is_valid` | JS WASM component loads via wasmtime |
| `rustacle-wasm-host` | `rust_fs_plugin_component_is_valid` | Rust WASM component loads via wasmtime |
| `rustacle-llm` | `token_cost_total` | Token cost arithmetic |
| `rustacle-llm` | `chat_delta_serialization` | ChatDelta JSON round-trip |
| `rustacle-llm` | `registry_profile_not_found` | Registry error on missing profile |
| `rustacle-llm-openai` | `parse_text_delta` | SSE text delta parsing |
| `rustacle-llm-openai` | `parse_done` | SSE done event parsing |
| `rustacle-llm-openai` | `parse_usage` | SSE usage event parsing |
| `rustacle-llm-openai` | `parse_tool_call_start` | SSE tool call start parsing |
| `rustacle-llm-local` | `probes_list_is_not_empty` | Local server probe list populated |
| `rustacle-agent` | `prompt_is_byte_identical_for_fixed_context` | Golden snapshot: deterministic prompt assembly |
| `rustacle-agent` | `prompt_deterministic_across_calls` | Two calls with identical context produce identical output |
| `rustacle-agent` | `changing_cwd_changes_only_env_layer` | Only env_context layer changes when cwd differs |
| `rustacle-agent` | `tools_filtered_by_permission` | Tools without permission are excluded from prompt |
| `rustacle-agent` | `empty_memory_omits_section` | Empty memory produces no memory section |
| `rustacle-agent` | `format_date_epoch` | Unix epoch formats to 1970-01-01 |
| `rustacle-agent` | `format_date_known` | Known timestamp formats correctly |
| `rustacle-agent` | `truncate_within_budget` | Text within budget passes through unchanged |
| `rustacle-agent` | `truncate_over_budget` | Over-budget text is truncated with marker |
| `rustacle-agent` | `filters_by_permission` | Tool schemas filtered by permission grants |
| `rustacle-agent` | `schemas_sorted_by_name` | Tool schemas sorted alphabetically for determinism |
| `rustacle-agent` | `cancel_propagates_to_child` | Cancel token propagates to child tokens |
| `rustacle-agent` | `child_cancel_does_not_propagate_up` | Child cancel doesn't affect parent |
| `rustacle-agent` | `flush_on_sentence_end` | Thought buffer flushes on sentence boundary |
| `rustacle-agent` | `no_flush_on_short_text` | Short text doesn't trigger flush |
| `rustacle-agent` | `flush_on_newline` | Thought buffer flushes on newline |
| `rustacle-agent` | `take_clears_buffer` | Take empties the thought buffer |
| `rustacle-agent` | `parsed_tool_call_json_roundtrip` | Tool call JSON parsing round-trip |
| `rustacle-agent` | `budget_check` | Budget guardrail limits enforced |
| `rustacle-agent` | `dispatch_registered_tool` | Registered tool dispatches correctly |
| `rustacle-agent` | `dispatch_unknown_tool_uses_placeholder` | Unknown tool uses placeholder |
| `rustacle-agent` | `names_sorted` | Tool dispatch table names are sorted |

```bash
cargo nextest run --workspace
cargo test -p rustacle-kernel -- permission
```

## Roadmap

| Sprint | Status | Goal |
|--------|--------|------|
| S0 — Foundation | Done | Workspace, Tauri shell, kernel, CI |
| S1 — IPC Bridge | Done | Type-safe IPC with Specta |
| S2 — Plugin System | Done | WASM plugin system + FS plugin + demo integration |
| S3 — Terminal | Done | PTY-backed terminal tabs + xterm.js UI |
| S4 — Agent | Done | Visible reasoning + LLM providers |
| S5 — Settings | Next | Zero-JSON settings UI |
| S6 — Multi-tab | Planned | Splits, tool redirection |
| S7 — Memory | Planned | Long-term memory + project context |
| S8 — Shipping | Planned | Hardening, telemetry, packaging |

See [`for_dev/roadmap.md`](./for_dev/roadmap.md) for detailed exit criteria.

## Contributing

Contributions are welcome! Please:

1. Check [open issues](https://github.com/robotoss/Rustacle/issues) or create a new one before starting work
2. Read the architecture docs in [`for_dev/`](./for_dev/) to understand the design
3. Follow the coding conventions in [`CLAUDE.md`](./CLAUDE.md)
4. Ensure all checks pass: `cargo fmt`, `cargo clippy`, `cargo nextest run`

## Issues & Support

- **Bug reports**: [GitHub Issues](https://github.com/robotoss/Rustacle/issues)
- **Feature requests**: [GitHub Issues](https://github.com/robotoss/Rustacle/issues)
- **Architecture docs**: [`for_dev/`](./for_dev/)

## License

This project is licensed under **CC BY-NC 4.0** (Creative Commons Attribution-NonCommercial 4.0 International).

You are free to use, share, and adapt this software for **non-commercial purposes** with attribution. Commercial use is not permitted without explicit permission.

See [LICENSE](./LICENSE) for details.
