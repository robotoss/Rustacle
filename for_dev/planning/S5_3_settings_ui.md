# S5.3 — Settings UI

## Goal
Build the Settings UI pages: Model Profiles, Providers, Permissions, Tools, Memory, Keybindings, Themes, and Plugins. Every setting must be editable from the UI — this is the Zero-JSON promise.

## Context
The Settings UI is a full-page view with a sidebar navigation and per-section forms. All fields are typed, validated, and round-trip through `rustacle-settings`. No setting should ever require the user to hand-edit a file. Form state is validated client-side (schema-level) and server-side (Rust crate rejects invalid payloads). API keys are stored via the OS keyring, never in SQLite.

## Docs to read
- `for_dev/ui_ux_manifesto.md` section 1 — Zero-JSON promise, section 1.2 wireframe for the Settings page layout.
- `for_dev/ui_ux_manifesto.md` section 4 — Command Palette and Keybindings.
- `for_dev/ui_ux_manifesto.md` section 5 — Themes.
- `for_dev/prompts_catalog.md` section 10 — Prompt customization rules.
- `for_dev/project_structure.md` — `ui/src/components/settings/` directory layout.

## Reference code
- `ui_ux_manifesto.md` section 1.2 — Settings wireframe.
- Internet: form libraries (react-hook-form/zod or @modular-forms/solid), live theme editors, CSS custom property editors.

## Deliverables
```
ui/src/components/settings/
├── SettingsPage.tsx        # Full-page layout with sidebar navigation
├── ModelProfiles.tsx       # CRUD profiles: provider, model, endpoint, key, temperature, tools
├── Permissions.tsx         # Per-plugin capability grants, revoke button
├── Keybindings.tsx         # vim/emacs/vscode bundles, conflict detection
├── Themes.tsx              # CSS token editor, live preview
├── PromptLayerEditor.tsx   # Advanced view: per-layer tabs, diff validation against SYSTEM_BASE safety sentences
└── ImportExport.tsx        # Stub — full implementation in S5_4
```

## Checklist
- [ ] All settings pages render without errors
- [ ] Model profile CRUD works (create, edit, delete)
- [ ] API key field stores via OS keyring (never in SQLite)
- [ ] Permission grants editable; changes invalidate broker cache immediately
- [ ] Keybinding bundles switchable between vim/emacs/vscode
- [ ] Keybinding conflict detection warns on duplicate bindings
- [ ] Theme tokens editable with live preview
- [ ] Prompt layer editor shows per-layer tabs
- [ ] Safety sentences in SYSTEM_BASE cannot be removed (save disabled with explanation)
- [ ] No setting requires file editing
- [ ] All form fields are typed and validated before submission

## Acceptance criteria
```bash
# UI compiles
pnpm --filter ui build

# Settings components render (component tests)
pnpm --filter ui test -- settings

# Rust settings crate compiles and passes tests
cargo check -p rustacle-settings
cargo test -p rustacle-settings

# Clippy clean
cargo clippy -p rustacle-settings -- -D warnings
```

## Anti-patterns
- Do NOT skip validation on forms — every field must be validated before save.
- Do NOT store API keys in SQLite — use the OS keyring exclusively.
- Do NOT allow removing safety-critical prompt sentences from SYSTEM_BASE.
- Do NOT create a `keybindings.json` file — keybindings live in the settings store.
- Do NOT allow saving an invalid model profile (e.g., missing provider or endpoint).
