# S5.1 — Settings Store (rustacle-settings)

## Goal
Create `rustacle-settings` — a SQLite-backed typed settings store with versioned schema, implementing the Zero-JSON philosophy where no config files exist.

## Context
The settings store is the backbone of the Zero-JSON philosophy. Every setting is persisted in SQLite, accessed via typed Rust APIs, and exposed to the UI via IPC. No config files. Schema is versioned for forward-compatible migrations. Change notifications allow other subsystems (e.g., permission broker) to react to settings changes.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 1 — Zero-JSON: philosophy and requirements.
- `for_dev/project_structure.md` section `rustacle-settings` — crate layout.
- `for_dev/tech_stack_2026.md` section 7 — Persistence: sqlx, SQLite, compile-time checked queries.

## Reference code
- Internet: [`sqlx`](https://docs.rs/sqlx/latest/sqlx/) with SQLite backend, compile-time checked queries via `sqlx::query!`, [`sqlx::migrate!`](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html) macro for embedded migrations.

## Deliverables
```
crates/rustacle-settings/
├── src/
│   ├── lib.rs          # Public API, re-exports
│   ├── store.rs        # SettingsStore: get/set typed values, batch updates, change notifications
│   ├── schema.rs       # Typed settings keys enum, default values, version tags
│   └── migrations.rs   # Re-export sqlx::migrate! macro
└── migrations/
    ├── 0001_init.sql       # Database initialization
    └── 0002_settings.sql   # Settings table: key, value (JSON), version, updated_at
```

## Checklist
- [ ] `SettingsStore` initializes SQLite database on first run (creates file if missing)
- [ ] `get::<T>(key)` returns typed value or the documented default
- [ ] `set::<T>(key, value)` persists to SQLite and emits a change notification
- [ ] Batch updates are transactional (all-or-nothing)
- [ ] Schema versioning supports forward-compatible changes (new keys with defaults)
- [ ] Migrations run automatically on startup via `sqlx::migrate!`
- [ ] Change notifications fire on the event bus for permission broker invalidation
- [ ] All settings have documented default values in the `schema.rs` enum
- [ ] No settings require manual file editing to configure
- [ ] Compile-time checked queries via `sqlx::query!` macros

## Acceptance criteria
```bash
# Crate compiles
cargo check -p rustacle-settings

# Unit tests pass (including migration tests)
cargo test -p rustacle-settings

# Clippy clean
cargo clippy -p rustacle-settings -- -D warnings

# Verify no config files are created
# (settings live only in SQLite, not JSON/TOML/YAML)
```

## Anti-patterns
- Do NOT use raw SQL strings — use `sqlx` compile-time checked queries exclusively.
- Do NOT store secrets in SQLite — secrets belong in the OS keyring (see S5_2).
- Do NOT create a settings file format (JSON, TOML, YAML) — SQLite is the only store.
- Do NOT skip default values — every setting must work out of the box without user configuration.
- Do NOT make migrations destructive — always preserve existing data when adding new schema versions.
