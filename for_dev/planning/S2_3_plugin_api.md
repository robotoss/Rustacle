# S2.3 — Plugin API (Traits & Types)

## Goal
Implement `rustacle-plugin-api` with the `RustacleModule` trait, `ModuleManifest`, `Capability` enum, and `ModuleError` — the host-side type definitions the kernel uses to interact with plugins.

## Context
This crate is a pure types-and-traits library. Plugin authors never implement `RustacleModule` directly — they export the WIT `module` interface, and `rustacle-wasm-host` adapts it into this trait. The kernel, permission broker, and module registry all depend on the types defined here. No runtime logic belongs in this crate.

## Docs to read
- `for_dev/architecture.md` section 4.3 — full Rust code for the `RustacleModule` trait.
- `for_dev/architecture.md` section 4.4 — plugin lifecycle (init, handle_command, on_event, shutdown).
- `for_dev/project_structure.md` — `rustacle-plugin-api` crate layout.

## Reference code
- `for_dev/architecture.md` section 4.3 contains the full Rust trait definition to transcribe.
- Internet: [`async_trait` docs](https://docs.rs/async-trait), [`thiserror` patterns](https://docs.rs/thiserror).

## Deliverables
```
crates/rustacle-plugin-api/src/
├── lib.rs           # re-exports all public types
├── module.rs        # RustacleModule async trait (init, handle_command, on_event, shutdown, export_state, import_state)
├── manifest.rs      # ModuleManifest, UiContributions, PanelDesc, PaletteEntry
├── capability.rs    # Capability enum: Fs(PathScope, FsMode), Net(HostPattern), Pty, Secret(String), LlmProvider
├── errors.rs        # ModuleError: Denied, InvalidInput, Trap, Internal (thiserror)
```

## Checklist
- [x] `RustacleModule` is an `async_trait` with methods: `init`, `call`, `on_event`, `shutdown`, `export_state`, `import_state`
- [x] All types derive `Clone`, `Debug`, `Serialize`, `Deserialize`
- [x] Types that cross the Tauri IPC boundary also derive `specta::Type`
- [x] `ModuleError` uses `thiserror` with `#[error(...)]` messages
- [x] `Capability` enum covers: `Fs(PathScope, FsMode)`, `Net(HostPattern)`, `Pty`, `Secret(String)`, `LlmProvider`
- [x] `PathScope` wraps a canonicalized path prefix
- [x] `FsMode` has `ReadOnly` and `ReadWrite` variants
- [x] `HostPattern` supports wildcard matching (e.g., `*.openai.com`)
- [x] `ModuleManifest` includes: `id`, `name`, `version`, `capabilities`, `ui_contributions`
- [x] `UiContributions` includes `panels: Vec<PanelDesc>` and `palette_commands: Vec<PaletteEntry>`
- [x] No runtime logic — only type definitions and trait declarations

## Acceptance criteria
```bash
# Compiles cleanly
cargo check -p rustacle-plugin-api

# No warnings
cargo clippy -p rustacle-plugin-api -- -D warnings

# Docs generate
cargo doc -p rustacle-plugin-api --no-deps
```

## Anti-patterns
- Do NOT add any runtime logic here — this crate is types and traits only.
- Do NOT make plugin authors implement `RustacleModule` directly — that is the wasm-host adapter's job.
- Do NOT use `String` for capabilities where a structured enum is appropriate.
- Do NOT skip `Serialize`/`Deserialize` derives — these types cross process boundaries.
