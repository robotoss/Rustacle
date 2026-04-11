# Rustacle — Claude Code Instructions

## What is this project

Rustacle is a local-first **agentic terminal** built on **Rust + Tauri v2**. Micro-kernel architecture: small kernel (lifecycle, IPC, permissions, event bus) + hot-swappable WASM plugins for all features (chat, terminal, agent, FS, memory, skills). Visible agent reasoning, zero-JSON config, capability-gated everything.

## Current status

**Sprint 2 complete.** Core infrastructure built: workspace, Tauri shell, kernel with plugin registry, type-safe IPC via tauri-specta, WASM plugin system (WIT contract, wasmtime host, Ed25519 signing), PermissionBroker, FS plugin as first WASM component, demo plugin with full UI integration. Next: **Sprint 3** (Terminal plugin).

## Architecture overview

- **Core crates** (`crates/`): `rustacle-kernel` (lifecycle, registry, permissions, bus), `rustacle-ipc` (typed commands/events/errors), `rustacle-plugin-api` (RustacleModule trait, Capability, Manifest), `rustacle-plugin-wit` (WIT contract), `rustacle-wasm-host` (wasmtime loader, linker), `rustacle-settings`, `rustacle-llm`, `rustacle-llm-{openai,anthropic,local}`, `rustacle-app` (Tauri binary)
- **WASM Plugins** (`plugins/`): `fs` (built via cargo-component, wasm32-wasip1), `chat`, `agent`, `memory`, `skills` — excluded from workspace, built separately
- **Native Plugins**: `terminal` (in workspace, implements RustacleModule directly)
- **Demo Plugin**: `DemoPlugin` in kernel crate — proves full UI→IPC→Kernel→Plugin pipeline
- **Frontend** (`ui/`): React 19 + Vite + Tailwind CSS v4, `bindings.ts` auto-generated via tauri-specta
- **IPC**: Tauri v2 commands (req/res) + events (streams), all typed via `specta`, CI-enforced sync

## Key documents

- `for_dev/concept.md` — vision and principles
- `for_dev/architecture.md` — micro-kernel, WIT contract, plugin system, event bus, permission broker
- `for_dev/project_structure.md` — full crate/file layout (authoritative)
- `for_dev/roadmap.md` — Sprint 0-8 plan with exit criteria
- `for_dev/planning/` — per-task briefs for sub-agents (S0_1 through S8_4)
- `for_dev/knowledge_base.md` — Rust patterns, error handling, DX guidelines, security, CI gotchas
- `for_dev/modularity.md` — core vs plugins, event bus, extension points
- `for_dev/agent_reasoning.md` — harness loop, prompt assembly, tool dispatch
- `for_dev/prompts_catalog.md` — system prompts verbatim
- `for_dev/mcp_and_models.md` — LLM providers, MCP integration
- `for_dev/cross_platform.md` — Windows/macOS/Linux specifics

## Non-negotiable rules

1. **Micro-kernel, not monolith.** Kernel = lifecycle + IPC + permissions + event bus. Everything else is a plugin.
2. **WASM-first plugins.** Native only where WASI can't reach (PTY today). Each native plugin has a migration note.
3. **Visible reasoning.** Every Thought/ToolCall/ToolResult/Answer streams as a typed event. No hidden reasoning.
4. **Zero-JSON UX.** Users never edit config files. Every setting has a UI control in the same PR.
5. **Capability-gated everything.** All I/O goes through the Permission Broker.
6. **Type-safe bridge.** `specta + tauri-specta` generates `bindings.ts`. Hand-written TS IPC types are bugs.
7. **Deterministic prompts.** `assemble_prompt()` must be byte-identical for identical inputs.
8. **Cancel-safe futures.** Every `.await` in a turn must be cancel-safe. Orphaned `tokio::spawn` is a bug.
9. **Local-first.** Any cloud feature must work with local models.
10. **Signed everywhere.** Every plugin and update is signed.

## Coding conventions

### Rust
- Edition 2024, `rust-version = "1.85"`
- `thiserror` in library crates, `anyhow` only at `main()`
- `tracing` for all logging (structured fields, not interpolation)
- Every `tokio::spawn` owned by a `JoinSet`; cooperative cancellation via `CancellationToken`
- Lock -> clone-out -> drop guard -> await (never hold guards across `.await`)
- `DashMap` over `Arc<Mutex<HashMap>>` where applicable
- `Bytes` for event payloads (zero-copy fan-out)
- `BTreeMap` or sorted vec in prompt assembly (determinism)
- No `.unwrap()` in non-test code without `// INVARIANT:` comment
- Clippy: `all = "warn"`, `pedantic = "warn"`
- Conventional commits: `feat(kernel): ...`, `fix(agent): ...`, `docs(for_dev): ...`

### TypeScript
- All types imported from generated `bindings.ts`, never hand-written
- Exhaustive match on `RustacleError.kind` (no default branch)
- Never `format!("{err:?}")` into user-facing strings

### Testing
- Unit: pure functions, `insta` snapshots for prompts
- Integration: kernel + one plugin (no Tauri window)
- Contract: per-plugin WIT binding tests
- E2E: Playwright + `tauri-driver`
- Property: `proptest` for parsers, canonicalizer, backpressure
- Golden prompt tests **mandatory** for any change to `plugins/agent/src/prompt/`

### WASM plugins
- Built via `cargo component build` with target `wasm32-wasip1`
- Excluded from workspace — each has standalone `Cargo.toml`
- WIT contract in `crates/rustacle-plugin-wit/wit/rustacle.wit`
- Must be Ed25519-signed before loading (`scripts/sign-plugin.sh`)

## Sprint plan (summary)

| Sprint | Status | Goal |
|--------|--------|------|
| S0 | Done | Workspace, Tauri shell, kernel skeleton, CI |
| S1 | Done | Type-safe IPC + Specta bridge |
| S2 | Done | Plugin API + WASM plugin system + FS plugin + demo integration |
| S3 | Next | Terminal plugin (native, PTY) |
| S4 | Planned | Agent plugin v1 (LLM, harness, visible reasoning) |
| S5 | Planned | Zero-JSON Settings UI + secrets/keyring |
| S6 | Planned | Multi-tab, splits, tool redirection |
| S7 | Planned | Memory plugin + project context |
| S8 | Planned | Hardening, telemetry, packaging |

Detailed task briefs are in `for_dev/planning/S{sprint}_{part}_{slug}.md`.

## After each sprint — mandatory checklist

These actions are **required** after completing every sprint:

1. **Update `README.md`** — use the `readme-updater` skill. Add new sections for major features (plugin system, IPC, terminal, agent, etc.). Keep cross-platform instructions (bash + PowerShell).
2. **Update test list in README** — list all tests by name in the Testing section with crate and description.
3. **Update `for_dev/roadmap.md`** — mark exit criteria as checked.
4. **Update `for_dev/planning/S*` files** — mark checklist items as done.
5. **Update `CLAUDE.md`** (this file) — keep "Current status" and "Sprint plan" table accurate.
6. **Regenerate `bindings.ts`** if IPC types changed — `cargo run -p rustacle-app --bin export_bindings`.
7. **Run full check** — `cargo clippy --workspace -- -D warnings && cargo test --workspace && cargo fmt --all -- --check`.

## Review checklist

- No new `.unwrap()` without `// INVARIANT:` comment
- No hand-written TS IPC types
- No new setting without a UI control
- New `.await` points reviewed for held guards
- New `tokio::spawn` has an owner in `JoinSet`
- Prompt changes include updated `insta` snapshot
- New capabilities wired through `PermissionBroker`
- New `pub` items in `rustacle-plugin-api` have doc comments
- CI green: tests, clippy, fmt, deny, bindings regen
- README updated with new features and tests

## References

- `refs/cc-src/` — Claude Code source (harness loop, tool dispatch patterns)
- `refs/acc/` — modular crate discipline, minimal Tauri shell
- `refs/claw-code/` — philosophy, parity discipline
- `rust-skills/` — Rust-specific skill system for Claude Code
