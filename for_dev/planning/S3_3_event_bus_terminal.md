# S3.3 — Event Bus Terminal Topics

## Goal
Wire `terminal.output` and `terminal.cwd` topics on the kernel event bus, bridging PTY events from the terminal plugin to the UI via Tauri events.

## Context
The event bus (architecture.md section 4.6) carries typed events between plugins and the UI. Terminal events use specific backpressure policies: `DropOldest` for output (high throughput — a slow subscriber must not block the PTY) and `CoalesceLatest` for cwd (only the latest directory matters). The bridge in `rustacle-app` forwards subscribed bus events as Tauri events to the webview.

## Docs to read
- `for_dev/architecture.md` section 4.6 — Event Bus design, topic registry, backpressure policies.
- `for_dev/project_structure.md` section `rustacle-kernel/bus/` — bus module layout.

## Reference code
- `for_dev/architecture.md` section 4.6 — Bus implementation details and policy definitions.
- Internet: [`tokio::sync::broadcast`](https://docs.rs/tokio/latest/tokio/sync/broadcast/) channel patterns, [`tokio::sync::watch`](https://docs.rs/tokio/latest/tokio/sync/watch/) for coalesce-latest semantics.

## Deliverables
```
crates/rustacle-kernel/src/bus/
└── topics.rs           # Updated: terminal.output (TerminalChunk, DropOldest),
                        #          terminal.cwd (CwdChange, CoalesceLatest)

crates/rustacle-ipc/src/events/
└── terminal.rs         # TerminalChunk, CwdChange event type definitions

crates/rustacle-app/src/
└── bridge.rs           # Forwards bus events → Tauri events for UI consumption
```

## Checklist
- [ ] `TerminalChunk` event type defined in `rustacle-ipc` (session id, bytes, sequence number)
- [ ] `CwdChange` event type defined in `rustacle-ipc` (session id, old path, new path)
- [ ] `terminal.output` topic registered with `DropOldest` backpressure policy
- [ ] `terminal.cwd` topic registered with `CoalesceLatest` backpressure policy
- [ ] Terminal plugin publishes `TerminalChunk` on `terminal.output` when PTY produces output
- [ ] UI receives terminal events via Tauri event listener (bridge forwarding)
- [ ] `terminal.cwd` updates when the shell working directory changes
- [ ] `DropOldest` policy: a slow subscriber does not block the publisher
- [ ] `CoalesceLatest` policy: subscriber only sees the latest cwd value
- [ ] Events are typed end-to-end: Rust structs → Specta/ts-rs bindings → TypeScript types

## Acceptance criteria
```bash
# Kernel compiles with new topics
cargo check -p rustacle-kernel

# IPC event types compile and have TS bindings
cargo check -p rustacle-ipc

# Bus integration tests pass
cargo test -p rustacle-kernel --test bus_integration

# Workspace compiles
cargo check --workspace
```

## Anti-patterns
- Do NOT use `BlockPublisher` policy for terminal output — it would stall the PTY read loop.
- Do NOT send raw bytes on the bus — use typed `TerminalChunk` with metadata.
- Do NOT bridge every bus event to Tauri — only forward topics the UI has actively subscribed to.
- Do NOT use `broadcast` channel for cwd — use `watch` or equivalent coalesce-latest primitive.
- Do NOT introduce new event bus abstractions — use the existing bus API from architecture.md section 4.6.
