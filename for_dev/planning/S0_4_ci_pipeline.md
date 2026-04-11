# S0.4 — CI Pipeline

## Goal
GitHub Actions CI matrix: Windows + macOS + Linux. On every PR: `cargo fmt`, `cargo clippy`, `cargo nextest`, `cargo deny`. All green.

## Context
CI is the project's guard rail. No PR merges without green CI. Set up once; each subsequent sprint adds checks.

## Docs to read
- `for_dev/tech_stack_2026.md` §9 (testing: nextest, insta), §10 (CI: Actions, sccache), §11 (deny, audit).
- `for_dev/cross_platform.md` §12 — per-OS test jobs matrix.
- `for_dev/knowledge_base.md` §3.3 — testing layers.

## Reference code
- `refs/claw-code/.github/` — example CI for a Rust + Tauri project (if available).
- Internet: [cargo-nextest CI guide](https://nexte.st/docs/ci/github-actions/), [cargo-deny GH action](https://github.com/EmbarkStudios/cargo-deny-action), [sccache GH action](https://github.com/Mozilla-Actions/sccache-action), [Tauri v2 CI guide](https://v2.tauri.app/distribute/ci-cd/).

## Deliverables

### `.github/workflows/ci.yml`
```yaml
name: CI
on: [push, pull_request]

env:
  CARGO_TERM_COLOR: always
  SCCACHE_GHA_ENABLED: true
  RUSTC_WRAPPER: sccache

jobs:
  check:
    strategy:
      matrix:
        os: [windows-2022, macos-14, ubuntu-22.04]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Mozilla-Actions/sccache-action@v0.0.6
      - uses: tauri-apps/tauri-action@v0  # installs system deps
        with:
          tauriScript: ''  # don't build, just install deps
      
      # Format
      - run: cargo fmt --all -- --check
      
      # Lint
      - run: cargo clippy --workspace -- -D warnings
      
      # Test
      - uses: taiki-e/install-action@nextest
      - run: cargo nextest run --workspace
      
      # Deny (licenses, advisories, duplicates)
      - uses: EmbarkStudios/cargo-deny-action@v2
        if: matrix.os == 'ubuntu-22.04'  # run once, not per-OS
```

### `deny.toml` (project root)

> **Note (cargo-deny v0.16+):** The `[advisories]` section removed per-field
> severity keys (`vulnerability`, `unmaintained`, `unsound`). The section is
> now minimal — just `ignore = [...]` for suppressed advisories. See
> [PR #611](https://github.com/EmbarkStudios/cargo-deny/pull/611).

```toml
[advisories]
# Per-field severity keys removed in v0.16+. Use ignore = [...] if needed.

[licenses]
allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-3.0", "Unicode-DFS-2016", "MPL-2.0"]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
allow-git = []
```

### `scripts/check-local.sh` (for local verification)
```bash
#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo nextest run --workspace
cargo deny check
echo "All checks passed!"
```

## Checklist
- [x] `.github/workflows/ci.yml` is created.
- [x] CI triggers on push and pull_request.
- [x] Matrix: windows-2022, macos-14, ubuntu-22.04.
- [x] `cargo fmt --check` — a CI step.
- [x] `cargo clippy -D warnings` — a CI step.
- [x] `cargo nextest run` — a CI step.
- [x] `cargo deny check` — a CI step (Ubuntu only, since it is platform-independent).
- [x] `sccache` is configured for caching.
- [x] `deny.toml` exists at root with license and advisory settings.
- [x] `scripts/check-local.sh` exists for local verification.
- [ ] CI is green on the current code. *(Not pushed to remote yet)*

## Acceptance criteria
- Push to main → CI runs on 3 OSes → all green.
- PR with `println!("test")` in `rustacle-kernel` → clippy warning → CI red.

## Anti-patterns
- Do NOT add e2e tests now — Playwright comes in S8.
- Do NOT add bindings regen check — that is S1.
- Do NOT cache `target/` directly — sccache is better.
- Do NOT use `continue-on-error: true` — CI must be strict.
