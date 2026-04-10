# ADR-0001 — UI Framework: Solid vs React

- **Status**: Proposed (to ratify at end of Sprint 0)
- **Date proposed**: 2026-04-09
- **Decider**: Lead Architect + UI engineer
- **Supersedes**: —
- **Superseded by**: —

## Context

Rustacle's UI lives in a Tauri v2 webview and drives three hot paths:

1. **Terminal widget** (`xterm.js`) — 60 fps scroll at 100k lines.
2. **Reasoning panel** — streaming token updates with sub-100 ms latency, thousands of cards.
3. **Settings UI** — large, deeply nested typed form (Zero-JSON philosophy — see `../ui_ux_manifesto.md` §1).

We need a framework that handles streaming without jank, renders large virtualized lists, and has a healthy forms + a11y story.

Both candidates have been used successfully with Tauri v2; both work with `bindings.ts` and `@tanstack/virtual`.

## Options

### A) SolidJS 1.x

**Pros**
- Fine-grained reactivity (signals). Token-streaming updates a single text node without rerendering a component tree.
- Lower allocations on hot paths; fewer GC spikes during long turns.
- Small runtime; bundle stays slim (helps cold-start target < 400 ms).
- First-class stores (`createStore`) are typed and ergonomic.
- Works cleanly with our `bindings.ts` since types are just TS.

**Cons**
- Smaller ecosystem; fewer prebuilt components (we'd build ours anyway to hit the UI manifesto).
- Hiring pool narrower than React.
- Less battle-tested a11y tooling compared to `react-aria`.

### B) React 19

**Pros**
- Huge ecosystem (virtualization, forms, a11y, theming).
- Bigger hiring pool.
- React 19's compiler closes some of Solid's perf gap.
- `react-aria` is the gold standard for a11y primitives.

**Cons**
- Heavier runtime; streaming paths allocate more.
- State management choice is another decision (Zustand? Jotai?).
- More effort to hit the cold-start budget.

## Decision criteria

| Criterion | Weight | Solid | React |
|---|---|---|---|
| Streaming perf (reasoning panel) | High | ✅ | ⚠️ (requires careful memoization) |
| Terminal widget integration | Medium | ✅ | ✅ |
| Ecosystem (a11y, forms, virtualization) | High | ⚠️ | ✅ |
| Cold-start bundle size | High | ✅ | ⚠️ |
| Hiring / onboarding | Medium | ⚠️ | ✅ |
| Typed IPC binding ergonomics | Medium | ✅ | ✅ |
| Settings UI depth | Medium | ✅ | ✅ |

## Proposed Decision

**Proceed with SolidJS** for Rustacle 1.0.

Rationale: the two hardest UI surfaces (streaming reasoning panel and terminal scrollback) are both allocation-sensitive; Solid's signal model matches them naturally. Cold-start budget is tight; Solid's runtime helps. A11y gaps are real but manageable by adopting `corvu` / `kobalte` (Solid's a11y primitives inspired by `react-aria`) and filling gaps ourselves.

Hiring risk is acknowledged and mitigated by (a) keeping UI code conventional (no exotic patterns), (b) strong types and tests, (c) thorough docs.

## Consequences

- `ui/` ships with Solid + Vite + Tailwind.
- State: `createStore` + signals, no separate state library.
- Forms: `@modular-forms/solid` + `zod`.
- Routing: `@solidjs/router` (minimal use; most of the app is modal/palette-driven).
- A11y primitives: `kobalte` + custom.
- Re-evaluation trigger: if by Sprint 6 we measure the reasoning panel missing its 60 fps budget on mid-tier hardware, we re-open this ADR.

## Ratification

Pending sign-off at Sprint 0 demo. Until ratified, `ui/` is untouched beyond a placeholder page in Sprint 0.

---
*Related: [README](../README.md) · [ui_ux_manifesto](../ui_ux_manifesto.md) · [tech_stack_2026](../tech_stack_2026.md)*
