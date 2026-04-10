# S8_1 — Telemetry & Observability

## Goal

Add opt-in OpenTelemetry export, opt-in Sentry crash reporting, and structured panic hooks with trace IDs.

## Context

Observability is opt-in — off by default, enabled from the Settings UI. OTLP export sends tracing spans to the user's own collector. Sentry captures panics with symbolicated stack traces. Structured panic hooks include trace IDs for correlation. No PII or secrets may appear in telemetry data.

## Docs to Read

- `for_dev/observability.md` — full observability strategy
- `for_dev/tech_stack_2026.md` — section 8 (Observability — tracing, OTLP, sentry, metrics)
- `for_dev/security.md` — no PII in telemetry

## Reference Code

- Internet: `opentelemetry-otlp` Rust crate docs
- Internet: `sentry` Rust crate docs
- Internet: `tracing-opentelemetry` layer setup
- Internet: `metrics-exporter-prometheus` crate docs

## Deliverables

```
crates/rustacle-app/src/
  telemetry.rs             # OTLP layer setup (opt-in), Sentry init (opt-in)
  panic_hook.rs            # Structured panic hook: stack trace, span context,
                           #   trace ID; writes to log and Sentry

src/components/settings/
  TelemetrySettings.tsx    # Settings UI toggles for OTLP + Sentry

docs/
  span_review_checklist.md # Review checklist: no PII, no secrets in span fields
```

## Checklist

- [ ] OTLP export off by default
- [ ] Enabling in Settings starts exporting spans
- [ ] Sentry off by default
- [ ] Panic hook captures trace ID + span context
- [ ] Panic hook logs structured output even without Sentry
- [ ] No PII in any span field (review checklist)
- [ ] No secret values in telemetry
- [ ] Settings UI has telemetry toggles with clear explanations
- [ ] Metrics (CPU, memory, IPC RTT) available via prometheus exporter (opt-in)

## Acceptance Criteria

```bash
# Unit tests pass
cargo test -p rustacle-app telemetry
cargo test -p rustacle-app panic_hook

# OTLP is off by default (no network calls on fresh start)
cargo test -p rustacle-app test_otlp_off_by_default

# Panic hook produces structured output without Sentry
cargo test -p rustacle-app test_panic_hook_no_sentry

# No PII grep (manual verification step)
rg -i "(email|password|token|secret|api.key)" crates/ --glob "*.rs" \
  | grep -i "span\|instrument\|tracing"
# Should return no matches in span/instrument annotations
```

## Anti-Patterns

- **Don't enable telemetry by default** — explicit user consent is required.
- **Don't send data without explicit user consent** — the toggle must be off on first launch.
- **Don't log PII or secrets in spans** — use the review checklist before merging.
- **Don't crash if OTLP endpoint is unreachable** — failures must be silent and non-blocking.
