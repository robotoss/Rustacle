# S8_3 — Signed Installers & Auto-Update

## Goal

Create signed installers for Windows (MSI), macOS (DMG), and Linux (AppImage/deb/rpm) via Tauri bundler, with auto-update support.

## Context

Shipping 1.0 requires signed, distributable packages. Tauri v2 bundler handles the heavy lifting. Auto-update uses the Tauri updater with Ed25519 signed manifests. Signing keys are separate from plugin-signing keys.

## Docs to Read

- `for_dev/tech_stack_2026.md` — section 10 (Build & Distribution — CI, packaging, updates, code signing)
- `for_dev/roadmap.md` — Sprint 8 deliverables

## Reference Code

- Internet: Tauri v2 bundler documentation
- Internet: Tauri updater documentation
- Internet: Authenticode signing guide
- Internet: Apple `notarytool` guide
- Internet: GitHub Actions code signing workflows

## Deliverables

```
crates/rustacle-app/
  tauri.conf.json          # Updated — bundler + updater config

.github/workflows/
  release.yml              # Release builds on Windows, macOS, Linux
                           #   Code signing per platform

scripts/
  release.sh               # Local release testing script

docs/
  signing-keys.md          # Key management documentation (no actual keys)
```

### Signing per platform

- **Windows**: Authenticode via signtool
- **macOS**: notarytool + stapling
- **Linux**: sigstore

### Update infrastructure

- Ed25519 keys for update manifests (separate from plugin keys)
- Updater endpoint configurable in Settings

## Checklist

- [ ] MSI installer on Windows, signed with Authenticode
- [ ] DMG on macOS, notarized with Apple
- [ ] AppImage + deb + rpm on Linux
- [ ] Auto-updater checks for updates on startup (configurable interval)
- [ ] Update manifests are Ed25519 signed
- [ ] Signing keys stored in GitHub Actions secrets
- [ ] Installer size < 15 MiB per platform
- [ ] First external alpha testers can install from artifacts

## Acceptance Criteria

```bash
# Build installers locally (unsigned)
cargo tauri build

# Verify MSI exists (Windows)
ls src-tauri/target/release/bundle/msi/*.msi

# Verify DMG exists (macOS)
ls src-tauri/target/release/bundle/dmg/*.dmg

# Verify AppImage exists (Linux)
ls src-tauri/target/release/bundle/appimage/*.AppImage

# CI workflow validates
act -j release --dryrun

# Updater config present
jq '.plugins.updater' crates/rustacle-app/tauri.conf.json
```

## Anti-Patterns

- **Don't share plugin-signing keys with update-signing keys** — they serve different trust domains.
- **Don't skip notarization** — macOS will quarantine unsigned apps and users cannot open them.
- **Don't hardcode update URLs** — make them configurable in Settings for self-hosted deployments.
- **Keep a fallback local-build path** — Apple notarization can be flaky; CI must not be blocked indefinitely.
