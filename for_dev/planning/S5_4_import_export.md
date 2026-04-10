# S5.4 — Settings Import/Export

## Goal
Implement typed settings import/export with diff preview, excluding secrets.

## Context
Users need portability — share settings across machines or team members. Import/export goes through a typed schema: drop a file in, the UI shows a diff, user clicks Apply. Secrets (API keys, tokens) are excluded from export. Wire format is an implementation detail, not a user interface. Capability grants in imported settings must require explicit per-plugin user confirmation before being applied.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 1 — Zero-JSON promise, import/export rules.
- `for_dev/architecture.md` section 6 — Threat model, settings import security considerations.
- `for_dev/project_structure.md` — `rustacle-settings/import_export.rs` location.

## Reference code
- Internet: JSON diff visualization libraries, file drop APIs in Tauri v2.

## Deliverables
```
crates/rustacle-settings/src/
└── import_export.rs        # export(): serialize all settings except secrets -> typed schema
                            # import(): parse + validate + return diff

ui/src/components/settings/
└── ImportExport.tsx         # File drop zone, diff preview, per-section toggle, Apply button

docs/adr/
└── ADR-0003-plugin-signing-key-distribution.md   # Pre-S8 decision record
```

## Checklist
- [ ] Export produces a typed file (JSON with schema version field)
- [ ] Import shows a diff before applying — never auto-applies
- [ ] Secrets are excluded from export
- [ ] Capability grants in import require per-plugin user confirmation
- [ ] Round-trip: export -> import -> export produces identical output (minus secrets)
- [ ] Invalid schema version shows a clear error message
- [ ] File drop works in Tauri webview
- [ ] Per-section toggle allows selective import
- [ ] ADR-0003 written for plugin signing key distribution

## Acceptance criteria
```bash
# Rust crate compiles
cargo check -p rustacle-settings

# Import/export round-trip test passes
cargo test -p rustacle-settings -- import_export

# UI compiles
pnpm --filter ui build

# Component tests for ImportExport
pnpm --filter ui test -- ImportExport

# Clippy clean
cargo clippy -p rustacle-settings -- -D warnings
```

## Anti-patterns
- Do NOT auto-apply imports — always show the diff first and require user confirmation.
- Do NOT include secrets (API keys, tokens) in exports.
- Do NOT silently grant capabilities from imported settings — each plugin's capability grants must be confirmed individually.
- Do NOT skip schema version validation on import.
