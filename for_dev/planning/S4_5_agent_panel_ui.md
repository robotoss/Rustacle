# S4.5 — Agent Panel UI

## Goal
Build the Agent Panel — a collapsible side panel that streams reasoning steps as typed cards in real time at 60fps.

## Context
The panel subscribes to `agent.reasoning` events and renders each step as a typed card: Thought, ToolCall, ToolResult, PermissionAsk, Answer, Error. Cards stream at 60fps with virtualized scrolling. The panel includes a Stop button for cancellation and a Cost badge showing live token usage.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 3 — Visible Agent Panel: card types, behavior, cost badge.
- `for_dev/ui_ux_manifesto.md` section 3.1 — Card wireframes.
- `for_dev/agent_reasoning.md` section 2 — Reasoning Event Schema.
- `for_dev/project_structure.md` section `ui/src/components/agent/` — expected component layout.

## Reference code
- Internet: [@tanstack/virtual](https://tanstack.com/virtual/latest) for virtualized scrolling, [ARIA live regions](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/ARIA_Live_Regions) for accessibility.

## Deliverables
```
ui/src/components/agent/
├── AgentPanel.tsx       # Collapsible panel, event subscription, layout
├── ThoughtCard.tsx      # Streaming text with partial updates
├── ToolCallCard.tsx     # Tool name, args, result summary, expandable payload
├── PermissionCard.tsx   # Capability display, Deny / Allow-once / Allow-always buttons
├── ReasoningCard.tsx    # Base card wrapper (shared styling, StepId, timestamp)
├── CostBadge.tsx        # Tokens in/out, cost estimate, duration

ui/src/state/
└── agent.ts             # Agent state store (steps, cost, active turn)
```

## Checklist
- [ ] Panel toggles with Ctrl/Cmd+J keyboard shortcut
- [ ] Thought cards stream tokens at 60fps using partial updates
- [ ] ToolCall cards show tool name, arguments, latency, and result summary
- [ ] Permission cards block the turn until the user clicks Deny, Allow-once, or Allow-always
- [ ] Answer cards render content as markdown
- [ ] Error cards show error message and retryable flag
- [ ] Stop button is visible during an active turn and cancels within 100ms
- [ ] Cost badge updates live with tokens in/out, cost, and duration
- [ ] Cards are virtualized beyond the viewport using @tanstack/virtual
- [ ] ARIA live region announces new steps for screen readers
- [ ] Cards with file references are clickable (jump-to-source)
- [ ] Panel state (open/closed, scroll position) persists across sessions

## Acceptance criteria
```bash
# Frontend builds without errors
cd ui && npm run build

# Type check passes
cd ui && npm run typecheck

# Component tests pass
cd ui && npm run test -- --filter agent

# Manual: open dev server, toggle panel with Ctrl+J, verify cards stream
cd ui && npm run dev
```

## Anti-patterns
- Do NOT re-render the entire panel on each token — append to the current ThoughtCard only.
- Do NOT skip virtualization — the panel must handle hundreds of steps without performance degradation.
- Do NOT show raw JSON to users — pretty-print tool arguments and results.
- Do NOT use polling for event updates — subscribe to the event stream.
- Do NOT block the UI thread during markdown rendering — use a web worker or async rendering if needed.
