# S4.4 — Tool Dispatch Table and Stock Tools

## Goal
Implement the `ToolDispatchTable` and stock tools: `fs_read`, `fs_write`, `fs_edit`, `grep`, `glob`, `bash`, `sub_agent`.

## Context
Tools implement the `Tool` trait defined in agent_reasoning.md. The dispatch table partitions calls into concurrent and serialized sets, fans out concurrent ones via `JoinSet`, and runs serialized ones sequentially. Each tool has strict input schema validation, required capabilities, and a golden test.

## Docs to read
- `for_dev/agent_reasoning.md` section 4 — Tool Dispatch: full trait definition, dispatch table, dispatch loop.
- `for_dev/agent_reasoning.md` section 4.3 — Dispatch pseudocode.
- `for_dev/tools_catalog.md` — every tool schema, description, validation rules, and behavior specification.

## Reference code
- `refs/cc-src/tools/` — FileReadTool, FileWriteTool, FileEditTool, GrepTool, GlobTool, BashTool, AgentTool patterns.

## Deliverables
```
plugins/agent/src/tools/
├── mod.rs          # Tool trait definition, re-exports
├── registry.rs     # ToolDispatchTable: register, lookup, partition, fan-out
├── fs_read.rs      # Binary detection, image summary, offset/limit paging
├── fs_write.rs     # 1MiB limit, no binary writes
├── fs_edit.rs      # Unique string match, replace_all option, diff output
├── grep.rs         # ripgrep library integration, max_matches limit
├── glob.rs         # Glob matching, sorted by mtime, result limit
├── bash.rs         # Delegates to terminal plugin via kernel, 6-layer validation
└── sub_agent.rs    # Spawns child harness with bounded budget
```

## Checklist
- [ ] `Tool` trait has methods: `schema()`, `validate()`, `concurrency()`, `required_capabilities()`, `call()`
- [ ] `ToolDispatchTable` partitions tool calls into concurrent and serialized sets
- [ ] `fs_read`: detects binary files, summarizes images, supports offset/limit paging
- [ ] `fs_write`: enforces 1MiB size limit, rejects binary content
- [ ] `fs_edit`: requires unique `old_string` match, supports `replace_all`, outputs diff
- [ ] `grep`: uses ripgrep library crate (not shell), enforces `max_matches`
- [ ] `glob`: returns results sorted by mtime, respects result limit
- [ ] `bash`: 6-layer validation (destructive warning, read-only mode, sed-i reject, path scope, interactive reject, allowlist)
- [ ] `sub_agent`: spawns child harness with bounded token/tool-call budget
- [ ] Each tool has at least one golden test
- [ ] Validation runs before permission check for every tool call

## Acceptance criteria
```bash
# Crate compiles
cargo check -p rustacle-plugin-agent

# All tool tests pass (including golden tests)
cargo test -p rustacle-plugin-agent -- tools

# Clippy clean
cargo clippy -p rustacle-plugin-agent -- -D warnings
```

## Anti-patterns
- Do NOT shell out for grep or glob — use `grep` and `glob` library crates directly.
- Do NOT skip validation before the permission check — validate first, then check capabilities.
- Do NOT allow bash to run interactive commands (e.g., `vim`, `less`, `ssh`) by default.
- Do NOT allow `fs_edit` to silently replace a non-unique match — error if `old_string` matches more than once (unless `replace_all` is set).
- Do NOT trust tool input without schema validation — every field must be validated against the declared schema.
