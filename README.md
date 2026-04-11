# Rustacle

> A next-generation **Agentic Terminal** built on Rust + Tauri v2. Micro-kernel at the core, hot-pluggable WASM modules at the edges, visible agent reasoning in the middle.

## Features

- **Visible Reasoning** — every Thought, ToolCall, and Answer streams as a typed event card in real time
- **Zero-JSON Config** — every setting lives in a UI control, never a config file
- **Micro-kernel Architecture** — small kernel (lifecycle, IPC, permissions, event bus) with plugin edges
- **WASM Plugin Sandbox** — capability-gated, signed, hot-swappable modules
- **Multi-tab Terminal** — per-tab agent context with tool-use redirection
- **Local + Cloud Models** — one trait, same UX for Ollama, LM Studio, OpenAI, Anthropic
- **MCP Support** — external tools and resources gated by the same permission broker
- **Cross-platform** — Windows, macOS, Linux via Tauri v2

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Tauri v2, tokio, tracing |
| Frontend | React 19, Vite, Tailwind CSS v4, TypeScript |
| IPC | tauri-specta (typed, auto-generated bindings) |
| Plugins | WebAssembly Component Model (wasmtime) |
| State | Zustand (UI), SQLite (settings/history) |
| Testing | nextest, insta, proptest, Playwright |

## Project Structure

```
rustacle/
├── crates/                   # Host-side Rust crates
│   ├── rustacle-kernel/      # Micro-kernel: lifecycle, bus, permissions
│   ├── rustacle-ipc/         # Typed IPC commands and events
│   ├── rustacle-plugin-api/  # Host-side plugin trait
│   ├── rustacle-plugin-wit/  # WIT contract surface
│   ├── rustacle-wasm-host/   # Wasmtime integration
│   ├── rustacle-settings/    # Zero-JSON settings store
│   ├── rustacle-llm/         # LLM provider abstraction
│   ├── rustacle-llm-openai/  # OpenAI-compatible provider
│   ├── rustacle-llm-anthropic/ # Anthropic provider
│   ├── rustacle-llm-local/   # Ollama / LM Studio provider
│   └── rustacle-app/         # Tauri v2 binary
├── plugins/                  # Plugin crates (WASM + native)
│   ├── fs/                   # File system (WASM)
│   ├── terminal/             # PTY terminal (native)
│   ├── chat/                 # Chat history (WASM)
│   ├── agent/                # Agent reasoning loop (WASM)
│   ├── memory/               # Long-term memory (WASM)
│   └── skills/               # User-defined tools (WASM)
├── ui/                       # React 19 + Vite + Tailwind CSS v4
└── for_dev/                  # Architectural documentation
```

## Prerequisites

- **Rust** stable (1.85+)
- **Node.js** 22+
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

# Run with debug logging
$env:RUSTACLE_LOG="debug"; cargo run -p rustacle-app
```

## Architecture

See [`for_dev/`](./for_dev/) for the full architectural documentation:

- [Concept & Vision](./for_dev/concept.md)
- [Architecture](./for_dev/architecture.md)
- [Roadmap](./for_dev/roadmap.md)
- [Knowledge Base](./for_dev/knowledge_base.md)

## Roadmap

| Sprint | Status | Goal |
|--------|--------|------|
| S0 — Foundation | Done | Workspace, Tauri shell, kernel, CI |
| S1 — IPC Bridge | Done | Type-safe IPC with Specta |
| S2 — Plugin API | In Progress | WASM plugin system + FS plugin |
| S3 — Terminal | Planned | PTY-backed terminal tabs |
| S4 — Agent | Planned | Visible reasoning + LLM providers |
| S5 — Settings | Planned | Zero-JSON settings UI |
| S6 — Multi-tab | Planned | Splits, tool redirection |
| S7 — Memory | Planned | Long-term memory + project context |
| S8 — Shipping | Planned | Hardening, telemetry, packaging |

## License

MIT
