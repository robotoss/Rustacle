# ADR-0002 — Foundation Freeze

- **Status**: Accepted
- **Date accepted**: 2026-04-11
- **Decider**: Lead Architect

## Context

Sprint 0 is complete. The workspace layout, Tauri shell, kernel skeleton, CI pipeline, and UI framework are established.

## Decision

Freeze the following directory layout as the project foundation:

```
rustacle/
├── Cargo.toml                    # workspace manifest
├── .cargo/config.toml            # build config
├── .github/workflows/ci.yml      # CI matrix
├── deny.toml                     # license + advisory checks
├── rustfmt.toml, clippy.toml     # linting config
├── for_dev/                      # architectural docs (canon)
├── crates/                       # host-side Rust crates (10)
│   ├── rustacle-kernel/          # micro-kernel
│   ├── rustacle-ipc/             # typed IPC bridge
│   ├── rustacle-plugin-api/      # host-side plugin trait
│   ├── rustacle-plugin-wit/      # WIT contract
│   ├── rustacle-wasm-host/       # wasmtime integration
│   ├── rustacle-settings/        # zero-JSON store
│   ├── rustacle-llm/             # provider abstraction
│   ├── rustacle-llm-openai/      # OpenAI provider
│   ├── rustacle-llm-anthropic/   # Anthropic provider
│   ├── rustacle-llm-local/       # local model provider
│   └── rustacle-app/             # Tauri v2 binary
├── plugins/                      # plugin crates (6)
│   ├── fs/, terminal/, chat/
│   ├── agent/, memory/, skills/
├── ui/                           # React 19 + Vite + Tailwind CSS v4
├── assets/, migrations/, keys/
├── tests/, scripts/
└── CLAUDE.md                     # AI assistant instructions
```

## Consequences

- Adding a new top-level directory requires an ADR.
- Renaming or removing a crate requires an ADR.
- Adding crates within `crates/` or `plugins/` is allowed without an ADR.
- UI framework is React 19 (per ADR-0001).

---
*Related: [ADR-0001](./0001-ui-framework.md) · [project_structure](../project_structure.md)*
