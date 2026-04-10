# S8_4 — Security Review & PARITY.md Checkpoint

## Goal

Conduct a security review of the WASM host, permission broker, and secret handling; create a `PARITY.md` checkpoint document.

## Context

Before shipping 1.0, every attack surface from `architecture.md` section 6 must be reviewed. The review covers WASM sandbox escapes, permission broker TOCTOU, secret leakage, LLM prompt injection defenses, and settings import safety. Results are triaged as P0-P3; P0/P1 must be fixed before shipping.

## Docs to Read

- `for_dev/architecture.md` — section 6 (Threat Model)
- `for_dev/security.md` — full STRIDE analysis
- `for_dev/knowledge_base.md` — section 4 (security)

## Reference Code

- Internet: OWASP top 10 for desktop applications
- Internet: WASM security best practices
- Internet: Tauri security documentation
- `refs/claw-code/PARITY.md` — checkpoint document format reference

## Deliverables

```
docs/
  security_review_v1.md    # Full review document (see scope below)
  PARITY.md                # Every shipped capability, format from refs/

for_dev/
  adr/
    ADR-0002-amended.md    # If architectural surface changed during hardening
```

### Review Scope

1. **WASM host** — fuel limits, memory limits, host imports audit, no ambient authority
2. **Permission broker** — TOCTOU between check and use, cache invalidation, scope escapes
3. **Secret handling** — keyring usage, redaction, log audit
4. **LLM prompt injection** — secret stripping from LLM requests, tool-use validation
5. **Settings import** — capability grant injection prevention

### Triage Matrix

| Priority | Definition                     | Action          |
|----------|--------------------------------|-----------------|
| P0       | Exploitable, data loss/leak    | Fix before ship |
| P1       | Exploitable, limited impact    | Fix before ship |
| P2       | Defense in depth gap           | Track, fix post |
| P3       | Theoretical, low likelihood    | Document only   |

## Checklist

- [ ] WASM host audit: no ambient authority, fuel limits enforced, memory limits enforced
- [ ] Permission broker: no TOCTOU between check and use, cache invalidation on settings change
- [ ] Secrets: nothing in SQLite, nothing in logs (verify with `rg`)
- [ ] Prompt injection: secrets stripped from LLM requests
- [ ] Settings import: grants require user click
- [ ] All P0 findings fixed
- [ ] All P1 findings fixed or mitigated with documented workaround
- [ ] `PARITY.md` created listing all shipped features
- [ ] ADR-0002 reviewed and amended if needed

## Acceptance Criteria

```bash
# No secrets in SQLite databases
sqlite3 data/*.db "SELECT * FROM settings WHERE key LIKE '%secret%' OR key LIKE '%token%' OR key LIKE '%api_key%';"
# Should return empty

# No secrets in log output
rg -i "(api.key|secret|password|token)" crates/ --glob "*.rs" \
  | grep -i "log\|info!\|warn!\|error!\|debug!\|trace!"
# Manual review — no secret values logged

# WASM fuel limits enforced
cargo test -p rustacle-wasm-host test_fuel_limit

# Permission broker TOCTOU test
cargo test -p rustacle-kernel test_permission_no_toctou

# PARITY.md exists and is non-empty
test -s docs/PARITY.md && echo "OK"

# All P0/P1 findings have associated test
cargo test security_
```

## Anti-Patterns

- **Don't skip the review because "it's just v1"** — security debt compounds and first impressions matter.
- **Don't self-review only** — get an independent reviewer (internal or external) for at least the P0/P1 findings.
- **Don't ship with known P0s** — these are blockers by definition.
- **Don't create PARITY.md as a wishlist** — it documents what actually shipped and works, not aspirations.
