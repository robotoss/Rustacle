# S4.3 — Thinking Loop (Harness)

## Goal
Implement the ReAct-style thinking loop (harness) in `plugins/agent` that streams thoughts, dispatches tools, and handles cancellation as the core agent execution engine.

## Context
The harness runs one generator loop per turn: assemble prompt, stream LLM response, parse deltas, dispatch tools, loop. Every step is a typed event streamed to the UI in real time. A cancel token enables clean stop within 100ms. This is the central execution engine that all agent interactions flow through.

## Docs to read
- `for_dev/agent_reasoning.md` section 1 — Thinking Loop: full diagram and pseudocode.
- `for_dev/agent_reasoning.md` section 1.1 — Cancel: cancellation semantics and token propagation.
- `for_dev/agent_reasoning.md` section 1.2 — Retry: transport-only retry policy.
- `for_dev/agent_reasoning.md` section 1.3 — Cost tracker: token counting and budget enforcement.
- `for_dev/agent_reasoning.md` section 5 — Harness Engineering Notes.

## Reference code
- `refs/cc-src/query.ts::queryLoop` (lines 241-1728) — the generator loop pattern: assemble, stream, parse, dispatch, repeat.
- `refs/cc-src/StreamingToolExecutor.ts` — concurrent vs serialized tool dispatch strategy.

## Deliverables
```
plugins/agent/src/harness/
├── mod.rs          # Harness struct, run_turn() entry point, re-exports
├── loop.rs         # Thinking loop: stream -> parse deltas -> emit steps -> dispatch -> loop
├── streaming.rs    # Partial thought flushing on sentence boundary or 80ms timeout
├── cancel.rs       # CancellationToken wiring, child tokens per tool call
└── dispatch.rs     # Placeholder — full dispatch implemented in S4_4
```

## Checklist
- [ ] Loop runs the cycle: assemble prompt -> stream LLM -> parse deltas -> dispatch tools -> repeat
- [ ] Text deltas become `Thought{partial: true}` steps streamed to the UI
- [ ] ToolUseStart / ToolUseDelta / ToolUseEnd accumulates tool calls correctly
- [ ] No tool calls in response -> emit Answer step -> end turn
- [ ] Stop button cancels the loop within 100ms via CancellationToken
- [ ] Transport errors retry with exponential backoff (max 3 attempts)
- [ ] Tool-semantic errors become observations fed back into the loop, not retries
- [ ] CostSample emitted on `agent.cost` topic after each LLM call
- [ ] Every step has a unique StepId (ULID)
- [ ] Budget guardrails enforced: max-tool-calls, max-duration, max-tokens
- [ ] Partial thoughts flush on sentence boundary or after 80ms, whichever comes first
- [ ] Child cancellation tokens propagate to individual tool calls

## Acceptance criteria
```bash
# Crate compiles
cargo check -p rustacle-plugin-agent

# Unit tests pass
cargo test -p rustacle-plugin-agent -- harness

# Clippy clean
cargo clippy -p rustacle-plugin-agent -- -D warnings
```

## Anti-patterns
- Do NOT nest loops — the harness is a single flat loop with dispatch as a sub-step.
- Do NOT retry tool-semantic errors — only transport/network errors are retryable.
- Do NOT block on tool dispatch — use `JoinSet` for concurrent tool execution.
- Do NOT emit events after I/O completes — emit before I/O to keep the UI responsive.
- Do NOT allocate a new channel per step — reuse the turn-scoped event sender.
