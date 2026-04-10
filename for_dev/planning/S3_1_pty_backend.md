# S3.1 — PTY Backend (portable-pty)

## Goal
Implement the PTY backend in `plugins/terminal` using `portable-pty` — spawn shells, resize, write input, and stream output as typed events on the bus.

## Context
The terminal plugin is the only native (non-WASM) plugin, whitelisted because WASI cannot yet spawn processes. It uses `portable-pty` for cross-platform PTY management (ConPTY on Windows, Unix PTY on macOS/Linux). It publishes `terminal.output` and `terminal.cwd` events on the event bus. The plugin implements `RustacleModule` directly (no WIT/WASM indirection).

## Docs to read
- `for_dev/architecture.md` section 4.1 — native fallback rationale and whitelisted plugin list.
- `for_dev/project_structure.md` section `plugins/terminal` — expected file layout and responsibilities.
- `for_dev/tech_stack_2026.md` section 3 — Terminal stack (portable-pty, vt100, xterm.js).
- `for_dev/cross_platform.md` — platform-specific considerations for shell spawning.

## Reference code
- Internet: [`portable-pty` crate docs](https://docs.rs/portable-pty) (WezTerm project), ConPTY Windows documentation, [`vt100` crate](https://docs.rs/vt100) for terminal state parsing.
- `refs/cc-src/tools/BashTool/` — shell execution patterns, process lifecycle, and signal handling.

## Deliverables
```
plugins/terminal/src/
├── lib.rs          # impl RustacleModule directly (init, shutdown, handle_command)
├── pty.rs          # PtySession: spawn, resize, write, read stream (async)
├── tabs.rs         # TabState: cwd, shell path, env vars, command history
└── parser.rs       # VT100 wrapping, TerminalChunk event construction
```

## Checklist
- [ ] Shell spawns correctly on Windows (ConPTY), macOS, and Linux
- [ ] Default shell is auto-detected (`$SHELL` on Unix, `comspec`/PowerShell on Windows)
- [ ] `write()` sends input bytes to the PTY child process
- [ ] `resize(cols, rows)` updates PTY dimensions
- [ ] Output streams as `TerminalChunk` events published to `terminal.output`
- [ ] `cwd` detection works (via shell integration sequences or `/proc/self/cwd` on Linux)
- [ ] `terminal.cwd` event published when working directory changes
- [ ] Tab state persists cwd, env, and shell path across operations
- [ ] Graceful shutdown kills the child process and cleans up PTY resources
- [ ] PTY read loop runs on a dedicated thread (not blocking the async runtime)

## Acceptance criteria
```bash
# Workspace compiles with the terminal plugin
cargo check --workspace

# Unit tests pass
cargo test -p rustacle-terminal

# PTY spawns and echoes (integration test)
cargo test -p rustacle-terminal --test pty_integration -- --nocapture
```

## Anti-patterns
- Do NOT use WASM for this plugin — it is a whitelisted native plugin.
- Do NOT block the async runtime on PTY reads — use `spawn_blocking` or a dedicated OS thread with a channel.
- Do NOT assume bash — detect the user's default shell via environment variables.
- Do NOT leak file descriptors — ensure PTY master/slave handles are closed on drop.
- Do NOT use `std::process::Command` directly — go through `portable-pty` for cross-platform correctness.
