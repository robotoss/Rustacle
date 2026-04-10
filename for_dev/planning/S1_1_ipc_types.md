# S1.1 — IPC Types Crate

## Goal
Create the `rustacle-ipc` crate with typed IPC commands, events, and the `RustacleError` enum. This crate is the single typed bridge between Rust and the UI.

## Context
All Tauri commands and events are defined in `rustacle-ipc`. No other crate may depend on Tauri's API version directly for type definitions. The crate exports pure data types and command signatures; it contains no business logic. The kernel and app crates consume these types to register handlers.

## Docs to read
- `for_dev/architecture.md` section 3 (IPC Layer) — defines the command/event contract.
- `for_dev/project_structure.md` section `rustacle-ipc` — expected file layout.
- `for_dev/tech_stack_2026.md` section 4 — serialization and type-export requirements.

## Reference code
- `refs/cc-src/` — IPC patterns used in a similar Tauri app.
- Internet:
  - [`specta` docs](https://docs.rs/specta) — derive macro for TS type generation.
  - [`tauri-specta` docs](https://docs.rs/tauri-specta) — Tauri integration for specta.
  - `serde` internally/externally tagged enums — for `RustacleError` serialization.

## Deliverables

### File tree
```
crates/rustacle-ipc/
├── Cargo.toml
└── src/
    ├── lib.rs              # re-exports all modules
    ├── errors.rs           # RustacleError tagged enum
    ├── commands/
    │   ├── mod.rs
    │   ├── plugins.rs      # stub: list_plugins, install_plugin, uninstall_plugin
    │   ├── settings.rs     # stub: get_settings, set_settings
    │   ├── agent.rs        # stub: send_prompt, cancel_agent
    │   ├── terminal.rs     # stub: exec_command, kill_process
    │   └── fs.rs           # stub: read_file, write_file, list_dir
    └── events/
        ├── mod.rs
        ├── agent.rs        # AgentStreamChunk, AgentStatusChanged
        ├── terminal.rs     # TerminalOutput, TerminalExited
        └── permission.rs   # PermissionRequest, PermissionResponse
```

### Type requirements
- All public types derive `Serialize, Deserialize, Clone, Debug, specta::Type`.
- `RustacleError` is an externally tagged enum with variants:
  - `NotFound { resource: String }`
  - `Denied { action: String, reason: String }`
  - `InvalidInput { field: String, message: String }`
  - `Internal { message: String }`
  - `PluginError { plugin_id: String, message: String }`
- Command functions are stubs returning `Result<T, RustacleError>` with `todo!()` or placeholder values.
- Event types are plain structs matching the topic registry in `architecture.md` section 4.6.

## Checklist
- [ ] `RustacleError` has all five variants listed above
- [ ] Each command module has at least one stub function with typed input/output
- [ ] Event types match `architecture.md` section 4.6 topic registry
- [ ] All public types derive `Serialize, Deserialize, specta::Type`
- [ ] `cargo check -p rustacle-ipc` passes with no errors
- [ ] `cargo clippy -p rustacle-ipc` passes with no warnings
- [ ] `cargo doc -p rustacle-ipc --no-deps` builds without warnings

## Acceptance criteria
```bash
cargo check -p rustacle-ipc
cargo clippy -p rustacle-ipc -- -D warnings
cargo doc -p rustacle-ipc --no-deps

# Verify error enum variants exist
grep -c "NotFound\|Denied\|InvalidInput\|Internal\|PluginError" crates/rustacle-ipc/src/errors.rs | grep -q "5"

# Verify specta derive on public types
grep -c "specta::Type" crates/rustacle-ipc/src/errors.rs
```

## Anti-patterns
- Do NOT implement command logic — that belongs in `rustacle-kernel`.
- Do NOT depend on the Tauri runtime crate in type definitions (only `serde`, `specta`).
- Do NOT use `String` where a dedicated enum is more appropriate (e.g., error variants, status codes).
- Do NOT add `#[tauri::command]` here — command registration happens in `rustacle-app`.
- Do NOT create god-structs; keep command inputs/outputs small and focused.
