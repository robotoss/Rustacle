# S5.2 — OS Keyring Integration

## Goal
Integrate the OS-native keyring for secret storage with a `SecretString` type that redacts on Debug, ensuring API keys and tokens never appear in SQLite, logs, or config files.

## Context
API keys and tokens must never appear in SQLite, logs, or config files. They live in the OS-native credential store (Windows Credential Manager, macOS Keychain, libsecret on Linux). `SecretString` wraps values with Debug redaction as defense-in-depth. This complements the settings store (S5_1) by handling the sensitive subset of configuration.

## Docs to read
- `for_dev/security.md` — secret handling requirements and threat model.
- `for_dev/tech_stack_2026.md` section 7 — keyring crate selection.
- `for_dev/architecture.md` section 6 — threat model: log lines containing secrets.
- `for_dev/observability.md` — tracing field redaction requirements.

## Reference code
- Internet: [`keyring`](https://docs.rs/keyring/latest/keyring/) crate docs, [`secrecy`](https://docs.rs/secrecy/latest/secrecy/) crate for the `SecretString` pattern, [Windows Credential Manager API](https://learn.microsoft.com/en-us/windows/win32/secauthn/credential-management).

## Deliverables
```
crates/rustacle-settings/src/
└── secrets.rs
    ├── SecretString     # Wraps String, Debug shows "***", Clone/Drop zeroing
    ├── KeyringStore     # get_secret, set_secret, delete_secret, list_keys
    └── fallback dialog  # Graceful error on Linux if libsecret is missing
```

## Checklist
- [ ] API keys are stored in the OS keyring, never in SQLite
- [ ] `SecretString` Debug output shows `"***"` instead of the actual value
- [ ] `SecretString` zeroes memory on Drop
- [ ] `get_secret` / `set_secret` / `delete_secret` work on Windows, macOS, and Linux
- [ ] Graceful error dialog on Linux when libsecret is not installed ("install libsecret-1-dev")
- [ ] `rg 'sk-' --glob '!for_dev' --glob '!keys/trusted_*'` finds nothing outside test fixtures
- [ ] Tracing fields go through a redactor — no secret values appear in log output
- [ ] Import/export workflows exclude secrets from the export payload
- [ ] `list_keys` returns key names only, never values

## Acceptance criteria
```bash
# Crate compiles
cargo check -p rustacle-settings

# Unit tests pass (secrets module)
cargo test -p rustacle-settings -- secrets

# Clippy clean
cargo clippy -p rustacle-settings -- -D warnings

# Verify no secrets leak into logs
rg 'sk-' --glob '!for_dev' --glob '!keys/trusted_*' --glob '!*.md'
# Expected: no matches
```

## Anti-patterns
- Do NOT store secrets in SQLite — that is the settings store's domain, not secrets.
- Do NOT log secret values at any tracing level — always redact.
- Do NOT skip zeroing memory on Drop — use `zeroize` or manual zeroing.
- Do NOT assume libsecret is available on Linux — detect and show a helpful error.
- Do NOT include secrets in settings import/export — secrets are keyring-only.
