# S6.2 — Tool Redirection

## Goal
Enable the agent to target specific tabs for tool calls, with UI drag-to-reroute before execution.

## Context
When the agent calls a shell tool, it targets a tab (default: active). The UI shows which tab is targeted. The user can drag the tool-call card onto another tab to reroute before the tool runs. This gives users control over where agent actions happen. Tool calls enter a brief pending state to allow rerouting before execution begins.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 2.2 — Tool-use redirection behavior.
- `for_dev/agent_reasoning.md` section 2 — `StepKind::ToolCall` has `tab_target` field.
- `for_dev/agent_reasoning.md` section 3.4 — cwd-aware and per-tab context.
- `for_dev/tools_catalog.md` section 6 — `bash` tool, `tab_target` field.

## Reference code
- `agent_reasoning.md` section 2 — ToolCall struct with `tab_target` field.
- `ui_ux_manifesto.md` section 2.2 — drag behavior description.

## Deliverables
```
ui/src/components/agent/
└── ToolCallCard.tsx         # Displays target tab indicator, draggable for reroute

ui/src/components/terminal/
└── TabBar.tsx               # Drop target: accepts tool-call card drops

plugins/agent/src/tools/
└── bash.rs                  # Reads tab_target from args, dispatches to targeted tab PTY

plugins/terminal/src/
└── tab_context.rs           # Per-tab agent context: last N commands + exit codes
```

## Checklist
- [ ] Agent tool calls show target tab indicator in the UI
- [ ] User can drag tool-call card to another tab to reroute
- [ ] Drag updates `tab_target` before tool executes
- [ ] Default target is the active tab
- [ ] `bash` tool runs in the targeted tab's PTY
- [ ] Per-tab context (commands, exit codes) is available to prompt assembly
- [ ] Tool calls enter pending state briefly to allow rerouting
- [ ] Reroute is blocked after execution starts

## Acceptance criteria
```bash
# Rust crate compiles
cargo check -p rustacle-plugin-agent
cargo check -p rustacle-plugin-terminal

# Tool redirection tests
cargo test -p rustacle-plugin-agent -- bash::tab_target
cargo test -p rustacle-plugin-terminal -- tab_context

# UI compiles
pnpm --filter ui build

# Component tests
pnpm --filter ui test -- ToolCallCard

# Clippy clean
cargo clippy -p rustacle-plugin-agent -- -D warnings
cargo clippy -p rustacle-plugin-terminal -- -D warnings
```

## Anti-patterns
- Do NOT execute a tool before the user has a chance to reroute (use a pending state or brief delay).
- Do NOT allow reroute after execution starts.
- Do NOT break existing single-tab behavior — when only one tab exists, redirection is invisible.
