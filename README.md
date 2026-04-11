# Rustacle

> A next-generation **Agentic Terminal** built on Rust + Tauri v2. Micro-kernel at the core, hot-pluggable WASM modules at the edges, visible agent reasoning in the middle.

## Features

- **Visible Reasoning** — every Thought, ToolCall, and Answer streams as a typed event card in real time
- **Zero-JSON Config** — every setting lives in a UI control, never a config file
- **Micro-kernel Architecture** — small kernel (lifecycle, IPC, permissions, event bus) with plugin edges
- **WASM Plugin Sandbox** — capability-gated, Ed25519-signed, hot-swappable modules
- **Type-safe IPC** — auto-generated TypeScript bindings from Rust types via tauri-specta
- **Permission Broker** — every plugin capability is gated, cached, and user-controllable
- **Multi-tab Terminal** — per-tab agent context with tool-use redirection
- **Local + Cloud Models** — one trait, same UX for Ollama, LM Studio, OpenAI, Anthropic
- **MCP Support** — external tools and resources gated by the same permission broker
- **Cross-platform** — Windows, macOS, Linux via Tauri v2

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

## Project Structure

```
rustacle/
├── crates/                       # Host-side Rust crates (workspace members)
│   ├── rustacle-kernel/          # Micro-kernel: lifecycle, bus, permissions
│   ├── rustacle-ipc/             # Typed IPC commands, events, RustacleError
│   ├── rustacle-plugin-api/      # RustacleModule trait, Capability, Manifest
│   ├── rustacle-plugin-wit/      # WIT contract (rustacle:plugin@0.1.0)
│   ├── rustacle-wasm-host/       # Wasmtime: loader, linker, fuel/memory limits
│   ├── rustacle-settings/        # Zero-JSON settings store (stub)
│   ├── rustacle-llm/             # LLM provider abstraction (stub)
│   ├── rustacle-llm-openai/      # OpenAI-compatible provider (stub)
│   ├── rustacle-llm-anthropic/   # Anthropic provider (stub)
│   ├── rustacle-llm-local/       # Ollama / LM Studio provider (stub)
│   └── rustacle-app/             # Tauri v2 binary + IPC commands
├── plugins/                      # Plugin crates
│   ├── fs/                       # File system (WASM component)
│   ├── terminal/                 # PTY terminal (native, workspace member)
│   ├── chat/                     # Chat history (WASM stub)
│   ├── agent/                    # Agent reasoning loop (WASM stub)
│   ├── memory/                   # Long-term memory (WASM stub)
│   └── skills/                   # User-defined tools (WASM stub)
├── ui/                           # React 19 + Vite + Tailwind CSS v4
│   ├── bindings.ts               # Auto-generated from Rust (DO NOT EDIT)
│   └── src/                      # Components, App.tsx
├── keys/                         # Trusted Ed25519 public keys for plugin signing
├── scripts/                      # Dev scripts
└── for_dev/                      # Architectural documentation (canon)
```

## Plugin System

Rustacle uses a **WASM Component Model** plugin architecture. Every feature (chat, agent, FS, memory) is a plugin compiled to `.wasm` and loaded by the host at runtime.

### How plugins work

1. **Contract** — plugins implement a WIT interface defined in `crates/rustacle-plugin-wit/wit/rustacle.wit`
2. **Host functions** — the host provides `fs-read`, `fs-write`, `net-fetch`, `secret-get`, `llm-stream`, `publish`, `log` to plugins
3. **Capabilities** — each plugin declares required capabilities (Fs, Net, Pty, Secret, LlmProvider) in its manifest
4. **Permission Broker** — every capability use is checked against user grants via `PermissionBroker`
5. **Signing** — all `.wasm` files must be Ed25519-signed; unsigned plugins are refused at load time

### Building WASM plugins

```bash
# Prerequisites
rustup target add wasm32-wasip1
cargo install cargo-component

# Build a single plugin
cd plugins/fs && cargo component build

# Build all WASM plugins
bash scripts/build-plugins.sh

# Sign a plugin
bash scripts/sign-plugin.sh target/wasm32-wasip1/debug/rustacle_plugin_fs.wasm
```

### Native plugins

`plugins/terminal` is a native plugin (PTY spawning requires OS-level APIs that WASI cannot yet express). It implements `RustacleModule` directly and is compiled as part of the workspace.

### Demo plugin (integration proof)

A built-in `DemoPlugin` proves the full pipeline works end-to-end:

```
UI (React) → tauri-specta IPC → Kernel PluginRegistry → DemoPlugin → response → UI
```

The demo plugin supports two commands:
- **`ping`** — returns "pong from plugin" with call count and timestamp
- **`echo`** — echoes input text with a `[demo]` prefix

Run the app and click **"Ping From Plugin"** or type in the echo input to verify the integration.

## IPC & Type Safety

All communication between Rust and the UI goes through typed IPC commands generated by `tauri-specta`:

- Rust types in `rustacle-ipc` are the single source of truth
- `ui/bindings.ts` is auto-generated — never hand-edit it
- `RustacleError` is exhaustively matched in TypeScript (no `default` branch)
- CI enforces that `bindings.ts` stays in sync with Rust types

```bash
# Regenerate bindings after changing IPC types
bash scripts/regen-bindings.sh            # macOS / Linux
cargo run -p rustacle-app --bin export_bindings  # any OS
```

## Prerequisites

- **Rust** stable (1.85+)
- **Node.js** 22+
- **cargo-component** (for WASM plugins): `cargo install cargo-component`
- **wasm32-wasip1 target**: `rustup target add wasm32-wasip1`
- **Platform dependencies**:
  - **Windows**: Visual Studio Build Tools (MSVC)
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`

## Getting Started

### 1. Clone the repository

```bash
git clone https://github.com/your-org/rustacle.git
cd rustacle
```

### 2. Install frontend dependencies

**macOS / Linux (bash, zsh):**
```bash
cd ui && npm install && cd ..
```

**Windows (PowerShell):**
```powershell
cd ui; npm install; cd ..
```

### 3. Build the frontend

**macOS / Linux:**
```bash
cd ui && npm run build && cd ..
```

**Windows (PowerShell):**
```powershell
cd ui; npm run build; cd ..
```

### 4. Run the app

```bash
cargo run -p rustacle-app
```

> **Important:** You must build the frontend (step 3) before running.
> Without `ui/dist/`, the build will fail.

## Development

**macOS / Linux:**
```bash
# Run all checks locally
bash scripts/check-local.sh

# Or individually:
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo nextest run --workspace

# Regenerate TypeScript bindings after changing Rust IPC types
bash scripts/regen-bindings.sh

# Build WASM plugins
bash scripts/build-plugins.sh

# Run with debug logging
RUSTACLE_LOG=debug cargo run -p rustacle-app
```

**Windows (PowerShell):**
```powershell
# Format check
cargo fmt --all -- --check

# Lint
cargo clippy --workspace -- -D warnings

# Tests
cargo nextest run --workspace

# Regenerate TypeScript bindings
cargo run -p rustacle-app --bin export_bindings

# Build WASM plugins
cd plugins/fs; cargo component build; cd ../..

# Run with debug logging
$env:RUSTACLE_LOG="debug"; cargo run -p rustacle-app
```

## Testing

The project uses multiple test layers:

| Layer | Tool | What it covers |
|-------|------|----------------|
| Unit | `cargo test` | Pure functions, permission logic, path scoping |
| Integration | nextest | Kernel + single plugin harness |
| Golden | insta | Prompt assembly snapshots (Sprint 4+) |
| Property | proptest | Path canonicalization, backpressure (Sprint 3+) |
| E2E | Playwright | Full Tauri app UI flows (Sprint 8) |

**Current tests (9):**

| Crate | Test | What it verifies |
|-------|------|-----------------|
| `rustacle-kernel` | `kernel_start_stop` | Kernel lifecycle (start, stop, cancellation) |
| `rustacle-kernel` | `registry_register_and_call` | Plugin registration + command routing |
| `rustacle-kernel` | `registry_call_unknown_plugin` | Error on unknown plugin |
| `rustacle-kernel` | `permission_allow_is_cached` | Grant caching in broker |
| `rustacle-kernel` | `permission_deny_is_not_cached` | Deny not cached (retryable) |
| `rustacle-kernel` | `invalidate_removes_grant` | Cache invalidation works |
| `rustacle-plugin-api` | `path_scope_contains` | FS path scope segment matching |
| `rustacle-plugin-api` | `host_pattern_exact` | Exact host matching |
| `rustacle-plugin-api` | `host_pattern_wildcard` | Wildcard `*.example.com` matching |

```bash
# Run all tests
cargo nextest run --workspace

# Run specific test
cargo test -p rustacle-kernel -- permission

# Run with output
cargo test -p rustacle-plugin-api -- --nocapture
```

## Architecture

See [`for_dev/`](./for_dev/) for the full architectural documentation:

- [Concept & Vision](./for_dev/concept.md) — what Rustacle is and why
- [Architecture](./for_dev/architecture.md) — micro-kernel, WIT, plugin system, event bus
- [Project Structure](./for_dev/project_structure.md) — every crate and file
- [Roadmap](./for_dev/roadmap.md) — Sprint 0–8 plan with exit criteria
- [Knowledge Base](./for_dev/knowledge_base.md) — Rust patterns, error handling, security

## Roadmap

| Sprint | Status | Goal |
|--------|--------|------|
| S0 — Foundation | Done | Workspace, Tauri shell, kernel, CI |
| S1 — IPC Bridge | Done | Type-safe IPC with Specta |
| S2 — Plugin API | Done | WASM plugin system + FS plugin |
| S3 — Terminal | Next | PTY-backed terminal tabs |
| S4 — Agent | Planned | Visible reasoning + LLM providers |
| S5 — Settings | Planned | Zero-JSON settings UI |
| S6 — Multi-tab | Planned | Splits, tool redirection |
| S7 — Memory | Planned | Long-term memory + project context |
| S8 — Shipping | Planned | Hardening, telemetry, packaging |

## License

MIT
