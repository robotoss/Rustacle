# ADR-0001 — UI Framework: React 19

- **Status**: Accepted
- **Date proposed**: 2026-04-09
- **Date accepted**: 2026-04-11
- **Decider**: Lead Architect
- **Supersedes**: —
- **Superseded by**: —

## Context

Rustacle's UI lives in a Tauri v2 webview and drives three hot paths:

1. **Terminal widget** (`xterm.js`) — 60 fps scroll at 100k lines.
2. **Reasoning panel** — streaming token updates with sub-100 ms latency, thousands of cards.
3. **Settings UI** — large, deeply nested typed form (Zero-JSON philosophy).

## Decision

**React 19** with Zustand for state management, Tailwind CSS v4 for styling.

### Rationale

- **Ecosystem maturity**: `react-aria` for a11y, `@tanstack/virtual` for virtualization, `react-hook-form` + `zod` for the massive Settings UI — all production-grade.
- **tauri-specta integration**: React is the primary target for `tauri-specta` examples and testing.
- **React 19 compiler**: closes the performance gap with Solid for most UI surfaces via automatic memoization.
- **Zustand**: minimal, typed, works well with Tauri's async IPC pattern.
- **Developer pool**: larger community means easier onboarding and more resources.

### Trade-offs accepted

- Heavier runtime than Solid — mitigated by React 19 compiler and careful virtualization.
- Cold-start budget tighter — mitigated by code splitting and lazy loading.
- Streaming panel needs explicit optimization — `useSyncExternalStore` + Zustand subscriptions.

## Consequences

- `ui/` ships with React 19 + Vite + Tailwind CSS v4.
- State: Zustand stores with typed slices.
- Forms: `react-hook-form` + `zod`.
- A11y: `react-aria` components.
- Virtualization: `@tanstack/react-virtual`.
- Re-evaluation trigger: if by Sprint 6 reasoning panel misses 60 fps on mid-tier hardware.

---
*Related: [README](../README.md) · [ui_ux_manifesto](../ui_ux_manifesto.md)*
