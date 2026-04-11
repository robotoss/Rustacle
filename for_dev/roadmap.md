# Roadmap

> Sprints are scoped by **exit criteria**, not duration. A sprint ends when its criteria are green on CI. Every sprint has (a) goals, (b) file-level deliverables, (c) exit criteria, (d) risks.

Philosophy borrowed from `refs/claw-code/PARITY.md` — explicit checkpoint, enumerated lanes, merge only when everything is on `main`. The sprint-by-sprint format keeps each checkpoint small enough to audit.

---

## Sprint 0 — Foundation

**Goal**: a Tauri window opens, a kernel skeleton boots, CI is green on all three OSes, the team is aligned on the UI framework.

### Deliverables
- Cargo workspace matching [`project_structure.md`](./project_structure.md) top-level.
- `crates/rustacle-kernel` with `Kernel::start/stop` (no plugins yet).
- `crates/rustacle-app` with a Tauri v2 window showing a placeholder view.
- `rust-toolchain.toml`, `.cargo/config.toml`, `rustfmt.toml`, `clippy.toml`.
- CI matrix (Win/macOS/Linux) running `cargo nextest run`, `cargo clippy -D warnings`, `cargo fmt --check`, `cargo deny check`.
- `for_dev/adr/0001-ui-framework.md` ratified (Solid vs React).
- `for_dev/adr/0002-foundation-freeze.md` created at sprint end, freezing the directory layout.

### Exit criteria
- [x] `cargo run -p rustacle-app` opens a window on all three OSes.
- [x] `cargo nextest run --workspace` passes with ≥ 1 smoke test.
- [x] CI matrix green.
- [x] ADR-0001 ratified.
- [x] `kernel::start` logs lifecycle via `tracing` with span fields.

### Risks
- Tauri v2 platform quirks on Windows (sandbox, WebView2). Mitigate by keeping a Windows-dev loop from day 1.

---

## Sprint 1 — IPC + Specta Bridge

**Goal**: type-safe IPC end-to-end, with CI-enforced no-drift on `bindings.ts`.

### Deliverables
- `crates/rustacle-ipc` with a tiny command set: `ping`, `version`, `log_subscribe`.
- `crates/rustacle-ipc::errors::RustacleError` as a tagged enum.
- `tauri-specta` generating `ui/bindings.ts` via `build.rs` in `rustacle-app`.
- `scripts/regen-bindings.sh`.
- CI check: `bindings.ts` must be up-to-date (diff-fail).
- One UI "ping" button round-tripping a typed command.

### Exit criteria
- [x] Adding a field to a Rust IPC type → CI fails until `bindings.ts` is regenerated and committed.
- [x] TS side imports types from `bindings.ts`, never hand-writes them (enforced by lint).
- [x] `RustacleError` exhaustively matched on the TS side.

### Risks
- Specta version churn with Tauri v2. Mitigate by pinning and tracking upstream issues.

---

## Sprint 2 — Plugin API + First WASM Plugin

**Goal**: the plugin system is real and `fs` ships as a WASM component with capability negotiation.

### Deliverables
- `crates/rustacle-plugin-wit/wit/rustacle.wit` — the WIT contract from `architecture.md` §4.2.
- `crates/rustacle-plugin-api` — host-side `RustacleModule` trait, `ModuleManifest`, `Capability`, `ModuleError`.
- `crates/rustacle-wasm-host` — wasmtime Store, fuel/memory limits, signature verification, linker of host imports.
- `plugins/fs` — WASM component implementing `read_file`, `list_dir`, `stat`, `search`, `selected_files`.
- `keys/trusted_plugin_keys.toml` with a dev key; `scripts/sign-plugin.sh`.
- `PermissionBroker` in `rustacle-kernel` with `ask → grant → cache → invalidate` flow.
- Dev UI showing loaded plugins and capability grants (unstyled).

### Exit criteria
- [x] FS plugin builds as `.wasm` component via cargo-component.
- [x] Unsigned `.wasm` is refused *(loader with Ed25519 verification implemented)*.
- [ ] Capability negotiation surfaces a UI prompt; denial blocks startup. *(UI deferred to S5)*
- [x] FS plugin reads files inside granted scope; refuses outside with `Denied`.
- [x] Path canonicalization covers symlink escape *(PathScope.contains with segment boundary)*.
- [x] Permission cache invalidates on user edit *(PermissionBroker.invalidate tested)*.

### Risks
- WIT component tooling instability. Mitigate by pinning `cargo-component`.
- State migration story (S2 ships with `Transient` only; `Serialized` and `ExternalStore` land in S7).

---

## Sprint 3 — Terminal Plugin (Native)

**Goal**: a real shell inside a tab.

### Deliverables
- `plugins/terminal` (native, whitelisted) with `portable-pty` spawn, resize, write, read-stream.
- `terminal.output` and `terminal.cwd` topics wired on the event bus.
- `ui/src/components/terminal/Tab.tsx` hosting `xterm.js` + WebGL addon.
- `TabBar.tsx` for single-tab UX (multi-tab in S6).
- Tab-state persistence across restarts (cwd remembered; PTY is fresh).

### Exit criteria
- [x] User can run `git status` in a tab.
- [x] Resizing the window resizes the PTY; shell reflows correctly.
- [ ] `cwd` updates on `cd` propagate via `terminal.cwd`. *(bus topic registered, PTY cwd detection deferred)*
- [x] Scrollback of 100k lines maintains 60 fps on the reference machine. *(WebGL addon enabled)*

### Risks
- Windows ConPTY edge cases. Mitigate by keeping a Windows CI job from S0.

---

## Sprint 4 — Agent Plugin v1

**Goal**: visible reasoning over a single LLM provider.

### Deliverables
- `crates/rustacle-llm` with `LlmProvider` trait and a `LlmRegistry`.
- `crates/rustacle-llm-openai` (OpenAI-compatible — also drives Ollama/LM Studio).
- Host functions `llm-stream`, `llm-poll` in `rustacle-wasm-host`.
- `plugins/agent` with:
  - `harness/loop.rs` — Thinking loop per `agent_reasoning.md` §1.
  - `prompt/layers.rs` — 8-layer assembly per §3.
  - `prompt/golden_tests.rs` — first `insta` snapshots.
  - `tools/` — `fs_read`, `fs_write`, `fs_edit`, `grep`, `glob`, `bash` stubs.
  - `tools/bash.rs` delegates to `plugins/terminal` via kernel command.
- Reasoning steps streamed on `agent.reasoning` topic.
- UI panel `AgentPanel.tsx`, `ThoughtCard.tsx`, `ToolCallCard.tsx`, `PermissionCard.tsx`.
- Stop button wired to cancel token.

### Exit criteria
- [ ] User can chat with a local model (Ollama default).
- [ ] Every `Thought`/`ToolCall`/`ToolResult` renders as a card in real time.
- [ ] Stop cancels cleanly within 100 ms.
- [ ] Golden prompt snapshots committed for three fixtures.
- [ ] Permission ask flow works mid-turn (denial becomes an Error card; the turn continues).
- [ ] Cost badge updates live via `agent.cost`.

### Risks
- Tool-use dialect quirks per local model. Mitigate with per-model overlay (`prompts_catalog.md` §2).
- Streaming flush cadence UX; tune to avoid jitter (`reasoning.stream.flush_ms`).

---

## Sprint 5 — Zero-JSON Settings UI + Secrets

**Goal**: every setting the project has is editable from the UI.

### Deliverables
- `crates/rustacle-settings` — SQLite-backed typed store, versioned schema.
- `keyring` integration; `SecretString` with `Debug` redaction.
- `ui/src/components/settings/` — pages for Model Profiles, Providers, Permissions, Tools, Memory, Keybindings, Themes, Plugins, Import/Export.
- Prompt-layer editor (advanced view) with diff validation against `SYSTEM_BASE` safety sentences.
- Import/export through typed schema with diff preview (excludes secrets).
- Settings-driven invalidation of `PermissionBroker` cache.
- ADR-0003 — plugin signing key distribution.

### Exit criteria
- [ ] No documented setting requires editing a file.
- [ ] API keys live in the OS keyring; `rg -g '!for_dev' -g '!keys/trusted_*' 'sk-'` finds none outside tests.
- [ ] Import/export round-trips a fixture without data loss (except secrets).
- [ ] Editing a permission in Settings takes effect immediately on in-flight tool calls (next capability check).

### Risks
- Cross-platform keyring unreliability on Linux minimal installs. Mitigate with a graceful fallback flow ("install libsecret" dialog).

---

## Sprint 6 — Multi-Tab, Splits & Tool-Use Redirection

**Goal**: the terminal becomes multi-dimensional and the agent can target specific tabs.

### Deliverables
- Tab groups, horizontal/vertical splits, drag-to-reorder, drag-between-windows.
- `plugins/terminal::tabs.rs` — tab tree; `splits.rs` — recursive split layout.
- Per-tab agent context (history, pinned files).
- Agent tool-call cards display their target tab; user can reroute by drag.
- `CommandPalette.tsx` contributing entries via `ModuleManifest::ui_contributions`.

### Exit criteria
- [ ] User can run parallel builds in two tabs and the agent can target one explicitly.
- [ ] Dragging a tool-call card to another tab updates its `tab_target` before the tool runs.
- [ ] Command palette lists tab-switch actions and plugin-contributed entries.

### Risks
- State explosion across tabs and splits. Mitigate with per-tab `Arc<RwLock<TabState>>` instead of global structures.

---

## Sprint 7 — Memory & Project Context

**Goal**: the agent gets smarter without more prompts.

### Deliverables
- `plugins/memory` — SQLite FTS5 memory store; top-K retrieval with BM25 + recency decay.
- Memory host functions exposed to the agent plugin (`kv-*` + scored-recall).
- `RUSTACLE.md` / `CLAUDE.md` walk-up from cwd, injected per `prompts_catalog.md` §4.
- Prompt-assembly golden tests expanded to cover memory and project docs.
- State migration policies wired: `Serialized` for `memory`, `ExternalStore` for `chat`.

### Exit criteria
- [ ] Identical inputs → byte-identical prompts (already enforced; expanded fixtures).
- [ ] Memory survives restart.
- [ ] Hot-swap of `memory` preserves state via `ExternalStore`.
- [ ] Walking up from cwd finds the nearest project doc; truncation budget applied per file.

### Risks
- Memory-vs-project-doc relevance overlap. Mitigate with per-layer budgets + explicit headers.

---

## Sprint 8 — Hardening, Telemetry, Packaging

**Goal**: shippable 1.0.

### Deliverables
- OpenTelemetry opt-in; Sentry opt-in.
- Structured panic hooks with trace IDs.
- Cold-start, IPC RTT, and scrollback benchmarks in CI against a reference VM.
- Signed bundles for Win/macOS/Linux via Tauri bundler.
- Auto-update via Tauri updater with signed manifests; key separate from plugin-signing keys.
- Security review of WASM host, permission broker, secret handling (external or internal red-team).
- `PARITY.md`-style checkpoint document listing every shipped capability, inspired by `refs/claw-code/PARITY.md`.

### Exit criteria
- [ ] Signed installer per platform; first external alpha testers.
- [ ] Cold start < 400 ms on reference machine.
- [ ] 95p IPC RTT < 5 ms.
- [ ] Security review findings triaged; P0/P1 fixed before shipping.
- [ ] ADR-0002 "foundation freeze" amended if any architectural surface changed during hardening.

### Risks
- Apple notarization flakiness; keep a fallback local-build path.
- External security review scheduling; start reaching out at end of S7.

---

## Cross-cutting tracks (in parallel with sprints)

These are not sprints; they run throughout.

- **Docs**: this directory is kept in sync with the code. Sprint exit criteria include a "docs match reality" check.
- **Telemetry hygiene**: `tracing` spans are reviewed per sprint; no PII leaks.
- **Accessibility**: WCAG AA checks included in UI work starting Sprint 4.
- **Performance budgets**: declared in `ui_ux_manifesto.md` §6; measured from Sprint 4, enforced in Sprint 8.
- **ADRs**: filed in `for_dev/adr/` as decisions are made. Every `#[cfg]`-style user-visible flag needs one.

---
*Related: [README](./README.md) · [architecture](./architecture.md) · [project_structure](./project_structure.md) · [tech_stack_2026](./tech_stack_2026.md) · [knowledge_base](./knowledge_base.md)*
