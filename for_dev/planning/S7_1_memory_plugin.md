# S7.1 — Memory Plugin

## Goal
Implement `plugins/memory` — a SQLite FTS5-backed long-term memory store with BM25 + recency decay scoring.

## Context
The memory plugin gives the agent persistent memory across sessions. It stores facts, preferences, and context that the agent can recall. Retrieval uses BM25 full-text search combined with recency decay, scored against the user turn text only (not conversation history). The plugin runs as a WASM module but stores data in host-side SQLite via host functions, ensuring data survives plugin hot-swaps.

## Docs to read
- `for_dev/agent_reasoning.md` section 3.2 principle #5 — Memory scored against user turn only.
- `for_dev/prompts_catalog.md` section 6 — Memory layer format.
- `for_dev/architecture.md` section 4.5 — State migration, ExternalStore for memory.
- `for_dev/project_structure.md` — `plugins/memory` directory layout.

## Reference code
- Internet: SQLite FTS5 documentation, BM25 scoring algorithm, recency decay functions (exponential decay).

## Deliverables
```
plugins/memory/src/
├── lib.rs              # wit-bindgen export, plugin entry point
├── store.rs            # SQLite FTS5 table: insert, delete, update
├── scoring.rs          # BM25 + recency decay, top-K retrieval
└── commands.rs         # remember(text), forget(id), recall(query, top_k)
```

Host functions: `kv-*` exposed to the agent plugin for memory access.

## Checklist
- [ ] Memory entries persist across restarts
- [ ] `remember(text)` stores entry with timestamp
- [ ] `forget(id)` removes entry
- [ ] `recall(query, top_k)` returns scored results
- [ ] BM25 scoring works on FTS5 index
- [ ] Recency decay: recent entries score higher
- [ ] `top_k` defaults to 6, configurable
- [ ] State migration: ExternalStore policy — hot-swap preserves data
- [ ] Memory survives plugin hot-swap
- [ ] Scored against user turn text only, not conversation history

## Acceptance criteria
```bash
# Rust crate compiles
cargo check -p rustacle-plugin-memory

# All memory tests pass
cargo test -p rustacle-plugin-memory

# Scoring tests specifically
cargo test -p rustacle-plugin-memory -- scoring

# Round-trip: remember -> recall returns the entry
cargo test -p rustacle-plugin-memory -- commands::round_trip

# Clippy clean
cargo clippy -p rustacle-plugin-memory -- -D warnings
```

## Anti-patterns
- Do NOT score against conversation history — only the user turn text.
- Do NOT use vector embeddings — FTS5 + BM25 is sufficient for v1.
- Do NOT store memory in WASM linear memory — use host-side SQLite via `kv-*` host functions.
- Do NOT lose memory data on plugin hot-swap — ExternalStore policy guarantees persistence.
