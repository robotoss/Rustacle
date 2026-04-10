# S7_2 — Project Docs Walk-Up & Injection

## Goal

Implement `RUSTACLE.md` / `CLAUDE.md` walk-up from cwd and inject project docs into the prompt assembly (layer 5).

## Context

The agent should automatically pick up project-level instructions. Walking up from the terminal's cwd, the system finds the nearest `RUSTACLE.md` or `CLAUDE.md` files and injects their contents into the prompt. Each file has a per-file token budget; if multiple docs exceed the total budget, the outermost is dropped first. This gives project authors a zero-config way to steer agent behavior.

## Docs to Read

- `for_dev/agent_reasoning.md` — section 3.2, principles #5-#6 (project docs walked once per turn)
- `for_dev/prompts_catalog.md` — section 4 (project docs layer format)
- `for_dev/project_structure.md` — overall layout

## Reference Code

- `refs/cc-src/constants/prompts.ts` — CLAUDE.md walk-up pattern used in Claude Code
- `for_dev/agent_reasoning.md` — section 3.3, layer 5 description

## Deliverables

```
crates/rustacle-kernel/src/
  project_docs.rs          # ProjectDocs struct, walk-up logic, truncation
  prompt/
    assemble.rs            # Updated — inject project docs at layer 5

crates/rustacle-kernel/src/turn_context.rs
  # ProjectDocs cached here (walked once per turn)

tests/
  golden/
    fixtures/project_docs/
      RUSTACLE.md           # Sample project doc
      nested/RUSTACLE.md    # Inner override
    snapshots/
      prompt_with_project_docs.snap
      prompt_with_budget_overflow.snap
```

## Checklist

- [ ] Walking from `/home/user/project/src/` finds `/home/user/project/RUSTACLE.md` and `/home/user/CLAUDE.md`
- [ ] Innermost file has higher priority (dropped last when over budget)
- [ ] Per-file truncation at 2000 tokens
- [ ] Total budget 8000 tokens — excess drops outermost first
- [ ] Results cached in `TurnContext` — not re-walked on tool loop iterations
- [ ] Golden test snapshot includes project doc layer
- [ ] Works on Windows (drive letter paths) and Unix

## Acceptance Criteria

```bash
# Unit tests pass
cargo test -p rustacle-kernel project_docs

# Golden tests pass (snapshots match)
cargo test -p rustacle-kernel golden

# Walk-up finds docs at multiple levels
cargo test -p rustacle-kernel test_walk_up_multi_level

# Budget overflow drops outermost
cargo test -p rustacle-kernel test_budget_overflow_drops_outermost

# Windows drive-letter path handling
cargo test -p rustacle-kernel test_walk_up_windows_root
```

## Anti-Patterns

- **Don't re-walk on every loop iteration** — cache in `TurnContext`. The filesystem walk happens once per turn.
- **Don't read files larger than budget without truncation** — always truncate to per-file limit before assembling.
- **Don't walk above filesystem root** — stop at `/` on Unix, at the drive root on Windows.
- **Don't include binary files** — only read UTF-8 text files named exactly `RUSTACLE.md` or `CLAUDE.md`.
