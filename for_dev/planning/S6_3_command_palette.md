# S6.3 — Command Palette

## Goal
Implement the Command Palette with fuzzy search, plugin-contributed entries, and contextual filtering.

## Context
The palette (`Ctrl/Cmd+K`) is the universal front for every user-invokable action. Plugins contribute entries via `ModuleManifest::ui_contributions.palette_entries`. Entries are fuzzy-searchable, recency-weighted, and contextual — inapplicable entries are hidden based on current state.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 4 — Command Palette and Keybindings.
- `for_dev/architecture.md` section 4.3 — `ModuleManifest::ui_contributions`.
- `for_dev/project_structure.md` — `ui/src/components/palette/` directory layout.

## Reference code
- Internet: VS Code command palette implementation, fuzzy search libraries (fuse.js, fzf algorithms).

## Deliverables
```
ui/src/components/palette/
├── CommandPalette.tsx       # Modal overlay, search input, result list, keyboard navigation
├── PaletteEntry.ts          # Entry type: id, label, keywords, action, context predicate
└── fuzzySearch.ts           # Fuzzy matching with recency weighting
```

Core entries: tab operations, settings, split actions.
Plugin entries loaded from `ModuleManifest::ui_contributions.palette_entries`.

## Checklist
- [ ] `Ctrl/Cmd+K` opens the palette
- [ ] Typing fuzzy-matches commands
- [ ] Arrow keys + Enter navigate and select
- [ ] Esc closes the palette
- [ ] Tab-switch actions listed
- [ ] Plugin-contributed entries appear
- [ ] Recency: recently used commands rank higher
- [ ] Context: inapplicable entries hidden (e.g., tab actions when no tabs open)
- [ ] Every keybinding maps to a palette entry
- [ ] Keyboard-only usable (ARIA roles and labels)

## Acceptance criteria
```bash
# UI compiles
pnpm --filter ui build

# Component and fuzzy search tests
pnpm --filter ui test -- CommandPalette
pnpm --filter ui test -- fuzzySearch

# Accessibility audit (no ARIA violations)
pnpm --filter ui test -- a11y
```

## Anti-patterns
- Do NOT hardcode entries — all entries come from a registry (core or plugin).
- Do NOT skip keyboard navigation — the palette must be fully keyboard-accessible.
- Do NOT show stale plugin entries after a plugin is unloaded.
- Do NOT ignore recency — recently used commands must rank higher in results.
