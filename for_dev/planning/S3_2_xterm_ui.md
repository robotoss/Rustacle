# S3.2 — Terminal UI (xterm.js + WebGL)

## Goal
Create the terminal UI component using `xterm.js` with WebGL rendering, hosted in a Tauri webview tab, receiving PTY output via events and sending keystrokes back to the backend.

## Context
The frontend terminal renders PTY output received via Tauri events. It uses xterm.js with the WebGL addon for GPU-accelerated rendering and the fit addon for auto-sizing to the container. Keyboard input goes back to the PTY via Tauri commands (`write_pty`). This sprint implements a single-tab terminal; multi-tab support comes in S6.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 2 — Multi-Window Terminal layout and interaction model.
- `for_dev/ui_ux_manifesto.md` section 6 — Performance targets (60fps at 100k lines scrollback).
- `for_dev/project_structure.md` section `ui/src/components/terminal/` — expected component layout.
- `for_dev/tech_stack_2026.md` section 3 — Terminal stack (xterm.js, WebGL addon, fit addon).

## Reference code
- Internet: [xterm.js docs](https://xtermjs.org/docs/), [`xterm-addon-webgl`](https://www.npmjs.com/package/xterm-addon-webgl), [`xterm-addon-fit`](https://www.npmjs.com/package/xterm-addon-fit), [Tauri v2 event listening from frontend](https://v2.tauri.app/develop/calling-rust/#listening-to-events).

## Deliverables
```
ui/src/components/terminal/
├── Tab.tsx             # xterm.js host element, WebGL addon init, fit addon
├── TabBar.tsx          # Single-tab bar, tab title derived from cwd
├── useTerminal.ts      # Hook: subscribe to terminal.output events, send keystrokes via write_pty command
└── terminal.css        # Theme integration with xterm.js theme object, layout styles
```

## Checklist
- [ ] Terminal renders in the Tauri webview using xterm.js
- [ ] WebGL addon is loaded and active (canvas renders via WebGL, not canvas2d)
- [ ] Typing sends keystrokes to PTY via `write_pty` Tauri command
- [ ] PTY output appears in real time via `terminal.output` event subscription
- [ ] Window resize triggers fit addon → `resize_pty` command → PTY resize
- [ ] 60fps scrollback performance at 100k lines (WebGL requirement)
- [ ] Tab title updates when `terminal.cwd` event fires
- [ ] Application theme colors are mapped to the xterm.js theme object
- [ ] Click-to-focus works correctly on the terminal element
- [ ] Terminal disposes cleanly on unmount (no leaked listeners or WebGL contexts)

## Acceptance criteria
```bash
# Frontend builds without errors
cd ui && npm run build

# Type check passes
cd ui && npm run typecheck

# Component renders (dev server smoke test)
cd ui && npm run dev
# Manual: open browser, verify terminal renders and accepts input
```

## Anti-patterns
- Do NOT use canvas2d rendering — the WebGL addon is mandatory for performance targets.
- Do NOT poll for output — use Tauri event subscription (`listen()`).
- Do NOT implement multi-tab logic yet — that is scheduled for S6.
- Do NOT bundle xterm.js theme colors inline — derive them from the application theme system.
- Do NOT create a new xterm.js Terminal instance on every re-render — initialize once and reuse.
