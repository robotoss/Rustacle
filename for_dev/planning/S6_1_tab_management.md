# S6.1 — Tab Management

## Goal
Implement multi-tab support with tab groups, horizontal/vertical splits, drag-to-reorder, and drag-between-windows.

## Context
Sprint 3 shipped a single-tab terminal. Now we add full tab management: multiple tabs, tab groups for project context, recursive splits within tabs, and drag operations. Each tab owns its own PTY, cwd, and agent context. Tab state (cwd, group membership) persists across restarts, but PTYs are freshly spawned.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 2 — Multi-Window Terminal: structure, behavior, keyboard shortcuts.
- `for_dev/project_structure.md` — `plugins/terminal/tabs.rs`, `plugins/terminal/splits.rs`.

## Reference code
- Internet: tab group UIs (Chrome, VS Code), recursive split layouts (tmux panes), drag-and-drop in React/Solid.

## Deliverables
```
plugins/terminal/src/
├── tabs.rs                 # Tab tree: create, close, reorder, group, per-tab state
└── splits.rs               # Recursive split tree: horizontal, vertical, resize ratios

ui/src/components/terminal/
├── TabBar.tsx              # Multi-tab bar, tab groups, drag-to-reorder
└── SplitTree.tsx           # Recursive split layout, draggable dividers
```

Keyboard shortcuts:
- `Ctrl+T` — new tab
- `Ctrl+W` — close tab
- `Ctrl+Shift+D` — split horizontal
- `Ctrl+D` — split vertical
- `Ctrl+1-9` — jump to tab by index

## Checklist
- [ ] Multiple tabs can be open simultaneously
- [ ] Each tab has independent PTY and cwd
- [ ] Tab groups visually group related tabs
- [ ] Drag-to-reorder tabs within a window
- [ ] Split a tab horizontally or vertically
- [ ] Nested splits work (split within split)
- [ ] Split dividers are draggable for resizing
- [ ] Keyboard shortcuts work for all tab operations
- [ ] Per-tab agent context (history, pinned files)
- [ ] Closing last tab in a split collapses the split
- [ ] Tab state persists across restarts (cwd remembered, PTY fresh)

## Acceptance criteria
```bash
# Rust crate compiles
cargo check -p rustacle-plugin-terminal

# Tab and split tests pass
cargo test -p rustacle-plugin-terminal -- tabs
cargo test -p rustacle-plugin-terminal -- splits

# UI compiles
pnpm --filter ui build

# Component tests
pnpm --filter ui test -- TabBar SplitTree

# Clippy clean
cargo clippy -p rustacle-plugin-terminal -- -D warnings
```

## Anti-patterns
- Do NOT use global state for tab data — use per-tab `Arc<RwLock<TabState>>`.
- Do NOT limit split depth arbitrarily — recursive splits must work to any reasonable depth.
- Do NOT forget to clean up PTY on tab close.
- Do NOT lose tab group membership on reorder.
