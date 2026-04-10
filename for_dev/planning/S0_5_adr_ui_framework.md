# S0.5 — ADR UI Framework & Scaffold

## Goal
Ratify ADR-0001 (Solid vs React decision), scaffold the chosen UI framework in `ui/`, and create ADR-0002 foundation freeze.

## Context
The UI framework decision blocks all frontend work. After this sprint, `ui/` contains a real framework (SolidJS or React), Tailwind CSS, and a working Vite dev server. ADR-0002 freezes the directory layout established during Sprint 0 so later sprints don't re-litigate structure.

## Docs to read
- `for_dev/adr/0001-ui-framework.md` — the existing draft; must be updated with a final decision and trade-off rationale.
- `for_dev/ui_ux_manifesto.md` sections 1-5 — design principles that constrain the framework choice.
- `for_dev/tech_stack_2026.md` section 6 — frontend stack requirements.
- `for_dev/project_structure.md` section `ui/` — expected directory layout.

## Reference code
- `refs/acc/acc-app/ui/` — UI scaffold patterns (Vite + framework + Tauri integration).
- Internet:
  - Tauri v2 + SolidJS setup guide
  - Tauri v2 + React setup guide
  - Vite configuration for Tauri (`@tauri-apps/cli` integration)
  - Tailwind CSS v4 installation with Vite

## Deliverables

### `ui/` scaffold
```
ui/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.ts      # or CSS-based config for Tailwind v4
├── postcss.config.js
├── src/
│   ├── main.tsx            # entry point
│   ├── App.tsx             # root component with placeholder layout
│   ├── index.css           # Tailwind directives
│   └── vite-env.d.ts
└── dist/                   # build output (gitignored)
```

### ADR updates
- `for_dev/adr/0001-ui-framework.md` — updated with final decision, trade-offs table, and "Accepted" status.
- `for_dev/adr/0002-foundation-freeze.md` — new file documenting the frozen directory layout after Sprint 0.

## Checklist
- [ ] Framework installed and listed in `ui/package.json`
- [ ] `cd ui && npm run dev` starts Vite dev server without errors
- [ ] `cd ui && npm run build` produces `ui/dist/` with bundled output
- [ ] Tailwind CSS classes render correctly in the placeholder App component
- [ ] `cargo run -p rustacle-app` launches the Tauri window showing the styled placeholder
- [ ] ADR-0001 has "Accepted" status with documented trade-offs
- [ ] ADR-0002 created with frozen directory tree

## Acceptance criteria
```bash
# Framework scaffold works
cd ui && npm install && npm run build
test -d ui/dist && echo "PASS: dist exists"

# Dev server starts (timeout after 5s = success, it stays running)
timeout 5 npm run dev || true

# Tauri app launches with UI
cargo run -p rustacle-app

# ADRs exist and are ratified
grep -i "accepted" for_dev/adr/0001-ui-framework.md
test -f for_dev/adr/0002-foundation-freeze.md && echo "PASS: ADR-0002 exists"
```

## Anti-patterns
- Do NOT add IPC calls — that is Sprint 1 work.
- Do NOT build real UI components — only a placeholder layout proving the stack works.
- Do NOT pick a framework without documenting trade-offs in ADR-0001.
- Do NOT install state management libraries yet.
- Do NOT configure SSR — Tauri uses client-side rendering only.
