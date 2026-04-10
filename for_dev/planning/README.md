# Sprint Planning Index

> Each file is a self-contained brief for a sub-agent. It contains everything needed: goal, context, documentation links, reference code pointers, checklist, and acceptance criteria.

## Naming convention

```
S{sprint}_{part}_{slug}.md
```

Example: `S0_1_workspace_setup.md` — Sprint 0, part 1, workspace setup.

## Execution order

### Sprint 0 — Foundation
| File | What it does | Depends on |
|---|---|---|
| [`S0_1_workspace_setup.md`](./S0_1_workspace_setup.md) | Cargo workspace, crate stubs, rust-toolchain | — |
| [`S0_2_tauri_shell.md`](./S0_2_tauri_shell.md) | Tauri v2 app, window, placeholder UI | S0_1 |
| [`S0_3_kernel_skeleton.md`](./S0_3_kernel_skeleton.md) | Kernel lifecycle, AppState, tracing | S0_1 |
| [`S0_4_ci_pipeline.md`](./S0_4_ci_pipeline.md) | GitHub Actions matrix, nextest, clippy, fmt, deny | S0_1 |
| [`S0_5_adr_ui_framework.md`](./S0_5_adr_ui_framework.md) | ADR-0001 ratification, UI scaffold | S0_2 |

### Sprint 1 — IPC + Specta Bridge
| File | What it does | Depends on |
|---|---|---|
| [`S1_1_ipc_types.md`](./S1_1_ipc_types.md) | rustacle-ipc crate, typed commands/events/errors | S0_3 |
| [`S1_2_specta_bridge.md`](./S1_2_specta_bridge.md) | tauri-specta bindings.ts generation, CI check | S1_1, S0_4 |
| [`S1_3_ui_roundtrip.md`](./S1_3_ui_roundtrip.md) | UI ping button → typed response | S1_2, S0_5 |

### Sprint 2 — Plugin API + First WASM Plugin
| File | What it does | Depends on |
|---|---|---|
| [`S2_1_wit_contract.md`](./S2_1_wit_contract.md) | WIT file, cargo-component setup | S1_1 |
| [`S2_2_wasm_host.md`](./S2_2_wasm_host.md) | wasmtime host, linker, fuel/memory limits | S2_1 |
| [`S2_3_plugin_api.md`](./S2_3_plugin_api.md) | RustacleModule trait, manifest, capability types | S2_2 |
| [`S2_4_permission_broker.md`](./S2_4_permission_broker.md) | Broker, cache, ask→grant flow | S2_3 |
| [`S2_5_fs_plugin.md`](./S2_5_fs_plugin.md) | plugins/fs as the first WASM component | S2_4 |

### Sprint 3 — Terminal Plugin
| File | What it does | Depends on |
|---|---|---|
| [`S3_1_pty_backend.md`](./S3_1_pty_backend.md) | portable-pty, spawn, resize, stream | S2_3 |
| [`S3_2_xterm_ui.md`](./S3_2_xterm_ui.md) | XTerm.js tab, WebGL, keyboard input | S3_1, S0_5 |
| [`S3_3_event_bus_terminal.md`](./S3_3_event_bus_terminal.md) | terminal.output, terminal.cwd topics | S3_1 |

### Sprint 4 — Agent Plugin v1
| File | What it does | Depends on |
|---|---|---|
| [`S4_1_llm_provider.md`](./S4_1_llm_provider.md) | LlmProvider trait, registry, OpenAI provider | S2_2 |
| [`S4_2_prompt_assembly.md`](./S4_2_prompt_assembly.md) | 8-layer assemble_prompt, golden tests | S4_1 |
| [`S4_3_harness_loop.md`](./S4_3_harness_loop.md) | Thinking loop, cancel, streaming | S4_2 |
| [`S4_4_tool_dispatch.md`](./S4_4_tool_dispatch.md) | ToolDispatchTable, stock tools (fs_read, grep, bash) | S4_3 |
| [`S4_5_agent_panel_ui.md`](./S4_5_agent_panel_ui.md) | AgentPanel, ReasoningCards, CostBadge | S4_3, S3_2 |

### Sprint 5 — Zero-JSON Settings + Secrets
| File | What it does | Depends on |
|---|---|---|
| [`S5_1_settings_store.md`](./S5_1_settings_store.md) | rustacle-settings, SQLite store, schema | S1_1 |
| [`S5_2_keyring.md`](./S5_2_keyring.md) | keyring integration, SecretString, redaction | S5_1 |
| [`S5_3_settings_ui.md`](./S5_3_settings_ui.md) | Settings pages: profiles, permissions, tools, themes | S5_1, S4_5 |
| [`S5_4_import_export.md`](./S5_4_import_export.md) | typed schema round-trip, diff preview | S5_3 |

### Sprint 6 — Multi-Tab + Tool Redirection
| File | What it does | Depends on |
|---|---|---|
| [`S6_1_tab_management.md`](./S6_1_tab_management.md) | Tab groups, splits, drag, multi-window | S3_2 |
| [`S6_2_tool_redirection.md`](./S6_2_tool_redirection.md) | tab_target in tool calls, drag-reroute UI | S6_1, S4_4 |
| [`S6_3_command_palette.md`](./S6_3_command_palette.md) | CommandPalette, plugin contributions | S6_1 |

### Sprint 7 — Memory + Project Context
| File | What it does | Depends on |
|---|---|---|
| [`S7_1_memory_plugin.md`](./S7_1_memory_plugin.md) | plugins/memory, FTS5, scored retrieval | S2_5 |
| [`S7_2_project_docs.md`](./S7_2_project_docs.md) | RUSTACLE.md walk-up, injection in prompts | S4_2 |
| [`S7_3_state_migration.md`](./S7_3_state_migration.md) | hot-swap state policies, ExternalStore | S7_1 |

### Sprint 8 — Hardening + Packaging
| File | What it does | Depends on |
|---|---|---|
| [`S8_1_telemetry.md`](./S8_1_telemetry.md) | OTLP opt-in, crash reporter, panic hooks | S5_1 |
| [`S8_2_benchmarks.md`](./S8_2_benchmarks.md) | Cold start, IPC RTT, scroll FPS, CI enforcement | S8_1 |
| [`S8_3_packaging.md`](./S8_3_packaging.md) | Signed bundles, auto-updater, per-OS | S8_2 |
| [`S8_4_security_review.md`](./S8_4_security_review.md) | WASM host audit, broker review, parity doc | S8_3 |

---

## How to use

Each file is a **self-contained prompt** for a sub-agent. Copy the file contents into the sub-agent's prompt. Each file contains:

1. **Goal** — what must be done (1-2 sentences).
2. **Context** — why, how it fits into the architecture.
3. **Docs to read** — links to `for_dev/*.md` with specific sections.
4. **Reference code** — what to look at in `refs/` (file:line) and on the internet.
5. **Deliverables** — concrete files/crates that must be created.
6. **Checklist** — what exactly must work.
7. **Acceptance criteria** — how to verify that the part is done.
8. **Anti-patterns** — what NOT to do.

---
*Related: [roadmap](../roadmap.md) · [architecture](../architecture.md) · [project_structure](../project_structure.md)*
