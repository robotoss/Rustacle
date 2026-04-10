# Tech Stack — 2026 Edition

> Every dependency earns its seat. If a crate isn't listed here, it doesn't go in `Cargo.toml` without an ADR. Versions noted as "currently best" — the exact pin is set at Sprint 0 and bumped through ADRs.

---

## 1. Core Runtime

| Layer | Choice | Version | Rationale | Alternatives rejected |
|---|---|---|---|---|
| Language | Rust stable, 2024 edition | current stable | Mature async, `let…else`, improved trait solver, stabilized `async fn in trait`. | Zig (ecosystem), C++ (safety) |
| MSRV policy | latest stable − 2 | — | Balances users on distros with CI freshness. Documented in `rust-toolchain.toml`. | Nightly (churn) |
| Desktop shell | **Tauri v2** | 2.x | Small bundles (< 15 MiB vs 150 MiB Electron), typed commands via Specta, mature plugin API, actively maintained. | Electron (size, memory), Wry-alone (no shell glue), Dioxus Desktop (immature) |
| Future-watch | Tauri v3 | preview | Migration isolated in `rustacle-ipc`. No v3 features depended on today. | — |
| Async runtime | `tokio` (multi-thread) | 1.x | De facto standard; event bus, IPC, PTY, wasmtime all integrate. | `smol` (smaller ecosystem), `async-std` (EOL track) |
| Sync primitives | `tokio::sync::*`, `parking_lot`, `dashmap` | — | `dashmap` for high-read-high-write registries; `parking_lot` only for non-yielding critical sections. | stdlib only (no sharded maps) |

---

## 2. Plugin Sandbox

| Layer | Choice | Rationale |
|---|---|---|
| WASM runtime | **`wasmtime`** + component model | Mature Component Model support, capability-based linking, fuel metering, memory limits, cross-platform. |
| Component tooling | `wit-bindgen`, `cargo-component` | Generates host/guest bindings from WIT interfaces; first-class WebAssembly ecosystem. |
| Signing | `ed25519-dalek` | Fast verify, small keys; public keys live in `keys/trusted_plugin_keys.toml`. |
| Native fallback loader | `libloading` (whitelisted only) | For `terminal` until WASI Preview 3 lands. Every native plugin is a compile-time feature of `rustacle-app`, not a runtime `.so`. |

**Sandbox hardening (applied per instance):**
- Fuel budget: 10 million instructions default, configurable per-plugin manifest.
- Memory limit: 64 MiB default.
- No ambient filesystem, no ambient network, no environ access. Only imports declared in `rustacle.wit`.
- One `Store<HostState>` per plugin instance (isolation).

**Why not Wasmer / WAMR / Extism?** Wasmer has weaker Component Model status; WAMR is AOT-focused; Extism is a higher-level abstraction over Wasmtime — we want direct control for permission gating.

---

## 3. Terminal

| Layer | Choice | Rationale |
|---|---|---|
| PTY | **`portable-pty`** | Cross-platform PTY spawn (Win32, macOS, Linux), battle-tested (used by WezTerm). |
| VT parser | `vt100` (primary) / `alacritty_terminal` (fallback for complex sequences) | `vt100` is small and easy to embed; Alacritty's parser if we hit edge cases. |
| Frontend render | `xterm.js` + `xterm-addon-webgl` + `xterm-addon-fit` | GPU-accelerated, ubiquitous ecosystem, themeable, works with Solid and React. |

**Why not embedding Alacritty directly?** Alacritty's window layer clashes with Tauri's webview; using only its parser (as a library) is a better fit.

---

## 4. IPC & Types

| Layer | Choice | Rationale |
|---|---|---|
| Typed bridge | **`specta` + `tauri-specta`** | Generates `bindings.ts` from Rust types. CI enforces no-drift. |
| Serialization | `serde`, `serde_json`, `bincode` (for event payloads) | Ecosystem default; bincode for hot-path event bus where size matters. |
| IDs | `ulid` | Sortable, compact, collision-free across distributed emits, human-readable in logs. |
| Schema versioning | `semver` + per-type `#[serde(tag = "v", content = "data")]` where evolution expected | Forward-compatible deserialization. |

---

## 5. LLM & Streaming

| Layer | Choice | Rationale |
|---|---|---|
| HTTP client | `reqwest` (rustls backend) | Async, TLS without OpenSSL headaches, well-maintained. |
| SSE | `eventsource-stream` | Tiny, stream-native, cancel-safe. |
| OpenAI-compatible | `async-openai` | Works for OpenAI, Ollama, LM Studio, vLLM, llama.cpp-server — anything speaking the OpenAI dialect. |
| Anthropic | Hand-rolled thin client in `rustacle-llm-anthropic` | Tool-use dialect differs enough that forcing it through a mega-crate hurts type safety. |
| Local autodiscovery | in `rustacle-llm-local` | Probes standard local ports at startup; auto-populates model profiles. |
| Tokenizer | `tiktoken-rs` + `tokenizers` (HF) | For budget-aware truncation in prompt assembly; model-specific tables. |

---

## 6. UI

**Framework: open question until ADR-0001 resolves in Sprint 0.** Both options below are documented to size the decision.

| Layer | Solid option | React option |
|---|---|---|
| Framework | SolidJS 1.x | React 19 |
| State | Solid stores (signals) | Zustand / Jotai |
| Router | `@solidjs/router` | `react-router` |
| Tradeoffs | Raw perf, fine-grained reactivity suits terminal streams; smaller ecosystem. | Bigger ecosystem, more hires; slightly more allocation on streaming paths. |

| Layer | Choice | Rationale |
|---|---|---|
| Bundler | **Vite** | Fast, Tauri-blessed, works with both Solid and React. |
| Styling | Tailwind CSS + CSS custom properties | Utility-first; CSS variables power the Theme Editor (see `ui_ux_manifesto.md` §5). |
| Terminal widget | `xterm.js` (see §3) | |
| Forms | `@modular-forms/solid` or `react-hook-form` + `zod` | Typed, validated, good a11y defaults. |
| Icons | `lucide` (SVG) | Consistent, themeable, tree-shakable. |
| Virtualization | `@tanstack/virtual` (framework-agnostic) | For the reasoning panel and large lists. |
| i18n | `@lingui/core` | Type-safe message catalogs. |

---

## 7. Persistence

| Layer | Choice | Rationale |
|---|---|---|
| DB | `sqlx` + SQLite | Compile-time checked queries, zero-ops embedded store, single-file backup. |
| Migrations | `sqlx::migrate!` | Lives in `migrations/`, runs on startup. |
| Secrets | `keyring` | OS-native credential store (Windows Credential Manager, macOS Keychain, Secret Service on Linux). No plaintext keys on disk. |
| Blobs | Local filesystem under app data dir, addressed by `BlobRef` | Keeps SQLite small; reasoning payloads can be MB-scale. |
| Full-text search (memory) | SQLite FTS5 | Built-in, good enough for top-K retrieval; no external index service. |

---

## 8. Observability

| Layer | Choice | Rationale |
|---|---|---|
| Logging | `tracing` + `tracing-subscriber` + `tracing-tree` (pretty dev output) | Structured, span-aware, async-friendly. |
| OTLP export | `opentelemetry-otlp` (optional, UI opt-in) | Power users ship spans to their own collector. Off by default. |
| Error model | `thiserror` for typed errors in libs, `anyhow` only at `main` boundary | Typed-first; see `knowledge_base.md` §2. |
| Crash reporting | `sentry` (optional, UI opt-in) | Native panic hook; symbol upload in release pipeline. |
| Metrics | `metrics` + `metrics-exporter-prometheus` (optional) | For Sprint 8 benchmarks. |

---

## 9. Testing

| Layer | Choice | Rationale |
|---|---|---|
| Unit / integration | **`cargo-nextest`** | Faster than stock test runner, better isolation, retries, JUnit output. |
| Snapshots | `insta` | Perfect for prompt-assembly golden tests and IPC bindings regression. |
| Property tests | `proptest` | For VT parser, path canonicalization, scope matching. |
| Fuzz | `cargo-fuzz` | VT parser, prompt assembler, WIT-binding deserializers. |
| UI e2e | **Playwright** via `tauri-driver` | Headless Tauri, realistic IPC, cross-platform. |
| Perf regression | `criterion` + checked-in baselines | For hot paths (bus, dispatch, prompt assembly). |
| Bench harness | `tauri-driver` + custom script | Cold-start, scroll fps, IPC RTT targets from `ui_ux_manifesto.md` §6. |

---

## 10. Build & Distribution

| Layer | Choice | Rationale |
|---|---|---|
| CI | GitHub Actions matrix (Windows / macOS / Linux) | Standard; cached `sccache`. |
| Packaging | Tauri bundler (msi, dmg, AppImage, deb, rpm) | First-class in Tauri v2. |
| Updates | Tauri updater + signed manifests | Auto-updates without a third-party service. Ed25519 signing keys separate from plugin-signing keys. |
| Code signing | Platform-native (Authenticode, notarytool, sigstore on Linux) | Via GitHub Actions secrets. |
| Reproducible builds | `cargo --locked` + pinned toolchain + pinned Docker base for Linux packaging | Reduces supply-chain surface. |

---

## 11. Dev experience utilities

| Tool | Purpose |
|---|---|
| `cargo-nextest` | Test runner (§9). |
| `cargo-component` | Build WASM components for plugins. |
| `cargo-deny` | License / advisory / duplicates audit, runs in CI. |
| `cargo-udeps` | Find unused deps. |
| `cargo-machete` | Same, alternative. |
| `cargo-audit` | RustSec advisories, runs in CI. |
| `wit-parser` | Validate WIT file on every PR. |
| `cargo-watch` | Dev loop. |
| `rustfmt` | Formatting (rules in `rustfmt.toml`). |
| `clippy` | Lints as errors in CI; `pedantic` warnings allowed locally. |

---

## 12. Deliberately not in the stack

| Not chosen | Why |
|---|---|
| Electron | Size, memory, bundle philosophy mismatch. |
| Nodejs for the host | Adds a second runtime; harms perf and security posture. |
| GraphQL / tRPC for IPC | Specta + tauri-specta is lighter and tighter. |
| Diesel | `sqlx` wins on async + compile-time query check. |
| A vector DB | Top-K with FTS5 + BM25 is enough for v1 memory. Revisit if quality requires it. |
| Lua / Python for scripting | Skills live in WASM for isolation and language neutrality. |

---

## 13. Version pin policy

Pins are recorded in `Cargo.toml` (workspace) and `ui/package.json`. Bumps require:
- CI green across the matrix.
- `cargo deny check` clean.
- Snapshot diffs reviewed if the bump touches prompt assembly or IPC types.
- An ADR for any major-version bump of a load-bearing dep (Tauri, wasmtime, sqlx).

---
*Related: [README](./README.md) · [architecture](./architecture.md) · [project_structure](./project_structure.md) · [roadmap](./roadmap.md)*
