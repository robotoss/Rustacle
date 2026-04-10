# S2.1 — WIT Contract & cargo-component Tooling

## Goal
Create the WIT contract file `rustacle.wit` in `crates/rustacle-plugin-wit/wit/` and set up `cargo-component` tooling so that the contract is parseable, bindgen-ready, and acts as the single source of truth for the plugin boundary.

## Context
The WIT file defines what plugins export (the `module` interface) and what the host provides (the `host` interface). Every WASM plugin component is validated against this contract at load time. The contract must match architecture.md section 4.2 exactly — any deviation requires a formal ADR. This is a prerequisite for S2.2 (wasm-host) and S2.5 (fs plugin).

## Docs to read
- `for_dev/architecture.md` sections 4.1–4.2 — WASM-first design rationale and the full WIT definition to transcribe.
- `for_dev/tech_stack_2026.md` section 2 — Plugin Sandbox stack (wasmtime, wit-bindgen, cargo-component versions).
- `for_dev/glossary.md` — canonical terminology for WIT types and concepts.

## Reference code
- Internet: [WIT specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md), [wit-bindgen docs](https://github.com/bytecodealliance/wit-bindgen), [cargo-component getting started](https://github.com/bytecodealliance/cargo-component), [wasmtime component model docs](https://docs.wasmtime.dev/api/wasmtime/component/).

## Deliverables
```
crates/rustacle-plugin-wit/
├── Cargo.toml              # updated with cargo-component config
└── wit/
    └── rustacle.wit         # full contract from architecture.md §4.2

scripts/
└── check-wit.sh            # validates WIT with wit-parser CLI
```

## Checklist
- [ ] `rustacle.wit` contains `package rustacle:plugin@0.1.0`
- [ ] `rustacle.wit` defines `interface types` with all shared types (context, command-result, event, capability, etc.)
- [ ] `rustacle.wit` defines `interface module` (what plugins export)
- [ ] `rustacle.wit` defines `interface host` (what the host provides to plugins)
- [ ] `rustacle.wit` defines `world plugin` composing module + host
- [ ] `scripts/check-wit.sh` runs `wasm-tools component wit` (or `wit-parser`) and exits 0
- [ ] `wit-bindgen rust` can generate Rust bindings from the WIT file without errors
- [ ] `Cargo.toml` has `[package.metadata.component]` pointing to the WIT directory

## Acceptance criteria
```bash
# WIT parses without errors
bash scripts/check-wit.sh

# Bindgen produces valid Rust (guest side)
wit-bindgen rust crates/rustacle-plugin-wit/wit/rustacle.wit --out-dir /tmp/wit-check

# Workspace still compiles
cargo check --workspace
```

## Anti-patterns
- Do NOT deviate from the WIT definition in architecture.md section 4.2 without filing an ADR first.
- Do NOT add host functions not listed in the contract.
- Do NOT use WIT preview1 syntax — use preview2 / component model syntax only.
- Do NOT hardcode version strings outside of the `package` declaration.
