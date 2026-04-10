# S7_3 — State Migration Policies

## Goal

Wire the full state migration policies: `Serialized` for memory plugin, `ExternalStore` for chat, and ensure hot-swap preserves state correctly.

## Context

Sprint 2 shipped with `Transient` only. Now we implement the remaining policies: `Serialized` (plugin implements `export_state`/`import_state` with max 1 MiB) and `ExternalStore` (state lives in SQLite, swap is trivial). The memory plugin uses `ExternalStore`; chat uses `ExternalStore`; fs uses `Transient`.

## Docs to Read

- `for_dev/architecture.md` — section 4.5 (Hot-swap & state migration — full sequence)
- `for_dev/architecture.md` — section 4.4 (lifecycle)
- `for_dev/project_structure.md` — `rustacle-wasm-host/state_migration.rs` entry

## Reference Code

- `for_dev/architecture.md` — section 4.5 has the 7-step hot-swap sequence

## Deliverables

```
crates/rustacle-wasm-host/src/
  state_migration.rs       # Updated — Serialized + ExternalStore policies

crates/rustacle-kernel/src/
  registry.rs              # Hot-swap sequence: load new -> export old state
                           #   -> import to new -> drain in-flight -> atomic swap -> drop old

crates/rustacle-wasm-host/src/
  manifest.rs              # StateMigrationPolicy enum in ModuleManifest

tests/
  state_migration/
    transient_test.rs      # State discarded on swap (fs plugin)
    serialized_test.rs     # export/import round-trip, 1 MiB limit
    external_store_test.rs # memory + chat hot-swap preserves data
```

## Checklist

- [ ] Transient: state discarded on swap (fs plugin test)
- [ ] Serialized: `export_state` -> `import_state` round-trip works
- [ ] Serialized: max 1 MiB enforced (oversized fails swap, keeps old)
- [ ] ExternalStore: memory plugin hot-swap preserves all data
- [ ] ExternalStore: chat plugin hot-swap preserves history
- [ ] In-flight commands drain with bounded timeout
- [ ] Atomic registry swap under `RwLock` write
- [ ] Failed `import_state` aborts swap and keeps old instance
- [ ] Event-bus subscriptions re-bound after swap

## Acceptance Criteria

```bash
# All state migration tests pass
cargo test -p rustacle-wasm-host state_migration
cargo test -p rustacle-kernel hot_swap

# Serialized round-trip
cargo test -p rustacle-wasm-host test_serialized_roundtrip

# Size limit enforcement
cargo test -p rustacle-wasm-host test_serialized_size_limit

# ExternalStore preserves data through swap
cargo test -p rustacle-kernel test_external_store_hot_swap

# Failed import aborts cleanly
cargo test -p rustacle-kernel test_failed_import_aborts
```

## Anti-Patterns

- **Don't lose state silently** — failed migration must abort the swap and keep the old instance running.
- **Don't skip the drain timeout** — in-flight commands must complete or time out before the swap proceeds.
- **Don't hold the registry write lock during state export** — export may be slow; only take the write lock for the atomic swap itself.
