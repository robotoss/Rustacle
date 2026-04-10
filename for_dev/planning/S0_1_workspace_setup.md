# S0.1 — Cargo Workspace Setup

## Goal
Create the Cargo workspace with all crate stubs. After this part, `cargo check --workspace` passes with zero errors.

## Context
Rustacle is a Rust/Tauri micro-kernel with WASM plugins. The workspace layout is defined in `for_dev/project_structure.md`. At this stage, crates contain only `lib.rs` / `main.rs` stubs — no logic, just compilation.

## Docs to read
- `for_dev/project_structure.md` — full crate tree, every file and its purpose.
- `for_dev/architecture.md` §2 — crate layout and rationale for the separation.
- `for_dev/tech_stack_2026.md` §1 — Rust edition, async runtime, MSRV.

## Reference code
- `refs/acc/acc-app/src-tauri/Cargo.toml` — example Tauri workspace layout.
- `refs/claw-code/rust/Cargo.toml` — example multi-crate workspace.
- Internet: [Cargo Workspaces (Rust Book)](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html), [Tauri v2 Getting Started](https://v2.tauri.app/start/).

## Deliverables

### Root files
- `Cargo.toml` — `[workspace]` with `members` listing all crates + plugins.
- `rust-toolchain.toml` — `channel = "stable"`, `edition = "2024"`.
- `.cargo/config.toml` — target dir, shared rustflags.
- `rustfmt.toml` — unified formatting style.
- `clippy.toml` — linter settings.

### Crates (each with `Cargo.toml` + `src/lib.rs` or `src/main.rs`)
```
crates/
├── rustacle-kernel/          # lib.rs: pub mod placeholder
├── rustacle-ipc/             # lib.rs: pub mod placeholder
├── rustacle-plugin-api/      # lib.rs: pub mod placeholder
├── rustacle-plugin-wit/      # wit/ dir only (empty rustacle.wit stub)
├── rustacle-wasm-host/       # lib.rs: pub mod placeholder
├── rustacle-settings/        # lib.rs: pub mod placeholder
├── rustacle-llm/             # lib.rs: pub mod placeholder
├── rustacle-llm-openai/      # lib.rs: pub mod placeholder
├── rustacle-llm-anthropic/   # lib.rs: pub mod placeholder
├── rustacle-llm-local/       # lib.rs: pub mod placeholder
└── rustacle-app/             # main.rs: fn main() {}
```

### Plugins (crate stubs)
```
plugins/
├── fs/           # lib.rs stub
├── terminal/     # lib.rs stub
├── chat/         # lib.rs stub
├── agent/        # lib.rs stub
├── memory/       # lib.rs stub
└── skills/       # lib.rs stub
```

### Other directories
```
ui/               # empty, populated in S0_5
migrations/       # empty
keys/             # empty
tests/            # empty
scripts/          # empty
assets/           # empty
```

## Checklist
- [ ] Root `Cargo.toml` is a workspace with all crates in members.
- [ ] Each crate has a `Cargo.toml` with correct `name`, `edition = "2024"`.
- [ ] `rust-toolchain.toml` is present.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --workspace -- -D warnings` passes.
- [ ] Inter-crate dependencies are declared as path dependencies (no versions yet).
- [ ] `rustacle-app` depends on `rustacle-kernel`, `rustacle-ipc`.
- [ ] `rustacle-kernel` depends on `rustacle-plugin-api`.
- [ ] Plugin crates depend on `rustacle-plugin-api` (empty dependency for now).
- [ ] No crate contains logic — stubs only.

## Acceptance criteria
```bash
cargo check --workspace  # exit 0
cargo fmt --all -- --check  # exit 0
cargo clippy --workspace -- -D warnings  # exit 0
```

## Anti-patterns
- Do NOT add external crate dependencies (tokio, serde, etc.) at this stage — only inter-crate path deps.
- Do NOT write logic — only struct/fn stubs for compilation.
- Do NOT create `Cargo.lock` manually — `cargo check` will generate it.
- Do NOT use `edition = "2021"` — the project uses 2024.
