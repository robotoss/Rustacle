# S1.2 — Specta Bridge (Auto-Generated TS Bindings)

## Goal
Wire `tauri-specta` to auto-generate `ui/bindings.ts` from Rust types, with CI enforcement that bindings stay in sync.

## Context
Type-safe IPC is non-negotiable. A hand-written TypeScript IPC type is a bug waiting to happen. The `bindings.ts` file is generated from Rust types via `build.rs` in `rustacle-app`. CI must fail if the checked-in bindings drift from the generated output.

## Docs to read
- `for_dev/architecture.md` sections 3.1-3.3 — IPC contract and type export flow.
- `for_dev/tech_stack_2026.md` section 4 — specta/tauri-specta version requirements.
- `for_dev/project_structure.md` section `rustacle-app` — build.rs location and responsibilities.

## Reference code
- Internet:
  - [`tauri-specta` v2 docs](https://github.com/oscartbeaumont/tauri-specta) — Builder API, export configuration.
  - [`specta` type export examples](https://docs.rs/specta) — how types are collected and exported.
  - Tauri v2 `build.rs` patterns — integrating code generation into the build pipeline.

## Deliverables

### Files to create or modify
```
crates/rustacle-app/
├── build.rs                    # calls tauri_specta::Builder, writes ui/bindings.ts
└── Cargo.toml                  # add tauri-specta, specta build-dependencies

ui/
└── bindings.ts                 # auto-generated, checked in, never hand-edited

scripts/
└── regen-bindings.sh           # standalone script: cargo build -p rustacle-app && copy bindings

.github/workflows/
└── ci.yml                      # add step: regen bindings, git diff --exit-code ui/bindings.ts
```

### `build.rs` behavior
1. Collect all commands from `rustacle-ipc::commands::*`.
2. Collect all event types from `rustacle-ipc::events::*`.
3. Call `tauri_specta::Builder::new()` with commands and events.
4. Export TypeScript bindings to `../ui/bindings.ts` (relative to `rustacle-app`).
5. Call `tauri_build::build()` as usual.

### `scripts/regen-bindings.sh`
```bash
#!/usr/bin/env bash
set -euo pipefail
cargo build -p rustacle-app
echo "Bindings regenerated at ui/bindings.ts"
```

### CI step
```yaml
- name: Check bindings are up-to-date
  run: |
    bash scripts/regen-bindings.sh
    git diff --exit-code ui/bindings.ts || {
      echo "ERROR: ui/bindings.ts is out of date. Run scripts/regen-bindings.sh and commit."
      exit 1
    }
```

## Checklist
- [ ] `cargo build -p rustacle-app` regenerates `ui/bindings.ts`
- [ ] Adding a field to a Rust IPC type causes `bindings.ts` to change on next build
- [ ] CI diff check catches stale bindings and fails the build
- [ ] TypeScript types in `bindings.ts` match Rust types exactly (verified by inspection)
- [ ] `scripts/regen-bindings.sh` works standalone on Linux, macOS, and Windows (Git Bash)
- [ ] `bindings.ts` has a generated-file header comment warning against manual edits
- [ ] `ui/` code can import from `bindings.ts` without TS errors

## Acceptance criteria
```bash
# Build regenerates bindings
cargo build -p rustacle-app
test -f ui/bindings.ts && echo "PASS: bindings.ts exists"

# Bindings contain expected types
grep "RustacleError" ui/bindings.ts && echo "PASS: error type exported"
grep "ping" ui/bindings.ts || echo "INFO: ping command not yet added (S1.3)"

# Regen script works
bash scripts/regen-bindings.sh

# CI check simulation
bash scripts/regen-bindings.sh
git diff --exit-code ui/bindings.ts && echo "PASS: bindings in sync"
```

## Anti-patterns
- Do NOT hand-write any TypeScript type that exists in Rust.
- Do NOT skip the CI diff check — stale bindings cause silent runtime failures.
- Do NOT put generation logic outside `build.rs` (the script just triggers a build).
- Do NOT import Tauri invoke directly — always go through the generated bindings.
- Do NOT suppress the generated-file header; it prevents accidental edits.
