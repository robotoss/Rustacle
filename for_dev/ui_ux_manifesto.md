# UI / UX Manifesto

> Audience: designers and frontend engineers. The rules below are binding; deviations require an ADR.

Framework decision: see [`adr/0001-ui-framework.md`](./adr/0001-ui-framework.md). Component layout: see [`project_structure.md`](./project_structure.md) §`ui/`.

---

## 1. Zero-JSON

**The user never opens a config file. Ever.**

Every setting — model endpoints, API keys, window layouts, keybindings, plugin toggles, prompt fragments, themes, permission grants — is editable from the Settings UI, backed by the typed `SettingsStore` in `rustacle-settings`. The on-disk form (SQLite) is an implementation detail.

### 1.1 Consequences

- **No "Edit config.json" menu item.** Anywhere.
- **No "paste this JSON snippet"** in any documentation.
- **Every new setting ships with its UI control in the same PR.** A setting without a control does not merge.
- **Import/export** exists for portability, but round-trips through a typed schema — users drop a file in, the UI shows a diff, they click Apply. The wire format is **not** a user interface.
- **Error messages never say "check your JSON."** They point at the exact UI field that's wrong.
- **The rule is about users, not engineers.** Developer-facing artifacts — `bindings.ts`, WIT files, Cargo.toml, CI configs — are untouched by Zero-JSON. This is a philosophy on user experience, not on build tooling.

Zero-JSON is the single most load-bearing UX decision in Rustacle. It is what separates this project from every "configurable" dev tool that quietly demands hand-edited YAML.

### 1.2 Settings UI layout (wireframe)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Rustacle · Settings                                         [ ✕ ]        │
├──────────────────────────────────────────────────────────────────────────┤
│ ┌──────────────┐  ┌──────────────────────────────────────────────────┐   │
│ │ Model        │  │ Model Profiles                        [ + New ]  │   │
│ │ Profiles     │  │ ┌────────────────────────────────────────────┐   │   │
│ │ ● default    │  │ │ Name         [ default              ]      │   │   │
│ │ ○ local-fast │  │ │ Provider     [ Anthropic           ▼]      │   │   │
│ │ ○ opus-heavy │  │ │ Model        [ claude-sonnet-4-6   ▼]      │   │   │
│ │              │  │ │ Endpoint     [ https://api....    ]       │   │   │
│ │ Providers    │  │ │ API key      [ ••••••••  ] [ Edit ]       │   │   │
│ │ Permissions  │  │ │ Temperature  [———○————————] 0.20           │   │   │
│ │ Tools        │  │ │ Max tokens   [ 8192 ]                      │   │   │
│ │ Memory       │  │ │ Persona      [                       ]     │   │   │
│ │ Keybindings  │  │ │                                             │   │   │
│ │ Themes       │  │ │ Enabled tools:                              │   │   │
│ │ Plugins      │  │ │   ☑ fs_read  ☑ fs_write  ☑ fs_edit          │   │   │
│ │ Import/Export│  │ │   ☑ grep     ☑ glob      ☑ bash             │   │   │
│ │              │  │ │   ☐ sub_agent                               │   │   │
│ │              │  │ │                                             │   │   │
│ │              │  │ │ System prompt (advanced) [ Edit layers... ] │   │   │
│ │              │  │ └────────────────────────────────────────────┘   │   │
│ └──────────────┘  └──────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────┘
```

Every field here is typed, validated, and round-trips through `rustacle-settings`. The "System prompt (advanced)" button opens a tabbed editor with one tab per prompt layer (see `prompts_catalog.md` §10).

---

## 2. Multi-Window Terminal

### 2.1 Structure

- **Windows** — top-level OS windows. Multiple supported; state syncs via the event bus so dragging a tab between windows is O(metadata).
- **Tabs** — inside a window. Each tab owns a PTY, `cwd`, shell, env, command history, and agent sub-context.
- **Tab groups** — visually grouped tabs sharing a project context (common root).
- **Splits** — recursive horizontal/vertical division within a tab. Each leaf is a PTY. Split ratios are draggable; Ctrl+drag snaps to grid.

### 2.2 Behavior

- **Directory-aware titles**: each tab's title is the nearest project root (git toplevel, `Cargo.toml`, `package.json`, `RUSTACLE.md`), followed by a short `cwd` suffix. Updates on `cd`.
- **Per-tab agent context**: each tab carries an independent memory of "what was tried here" (last N commands, exit codes, pinned files). The agent can reason about one tab in isolation.
- **Tool-use redirection**: when the agent issues a shell tool, it targets a tab (default: active). The UI shows a subtle arrow pointing at the target tab. The user can reroute by dragging the tool-call card onto another tab. On drop, the card's `tab_target` updates and the call proceeds.
- **Reattach after crash**: tabs are persisted with their PTY metadata; on restart, the user is offered "reattach" (spawn a new shell in the remembered cwd) or "discard".

### 2.3 Keyboard

Stock keymap (editable in Settings — §4):

| Binding | Action |
|---|---|
| `Ctrl/Cmd+T` | New tab |
| `Ctrl/Cmd+W` | Close tab |
| `Ctrl/Cmd+Shift+D` | Split horizontal |
| `Ctrl/Cmd+D` | Split vertical |
| `Ctrl/Cmd+1..9` | Jump to tab N |
| `Ctrl/Cmd+[/]` | Prev/Next tab |
| `Ctrl/Cmd+K` | Command palette |
| `Ctrl/Cmd+J` | Toggle Agent panel |
| `Ctrl/Cmd+.` | Stop current agent turn |

---

## 3. The Visible Agent Panel

A collapsible panel (default: right side, resizable) that streams the agent's reasoning in real time.

### 3.1 Card types

```
┌─ 🧠 Thought ──────────────────────────────────────────── 120ms ─┐
│ I need to find all TODO comments in src/. Grep looks like the   │
│ right tool here — it supports ripgrep globs.                    │
└─────────────────────────────────────────────────────────────────┘

┌─ ⚙ grep ─────────────────────────────── Tab 1: rustacle/src ────┐
│ { "pattern": "TODO", "path": "src", "type": "rust" }            │
├─────────────────────────────────────────────────────────────────┤
│ 47 matches in 12 files                            [ Expand ▾ ]  │
└─────────────────────────────────────────────────────────────────┘

┌─ 🛡 Permission Ask ──────────────────────────────────────────── ┐
│ The agent wants Fs write access to                              │
│ /home/k/projects/rustacle/src                                   │
│                        [ Deny ] [ Allow once ] [ Allow always ] │
└─────────────────────────────────────────────────────────────────┘

┌─ ✓ Answer ──────────────────────────────────────────────────── ┐
│ Found 47 TODOs. Hotspots:                                       │
│  • src/kernel/bus.rs:142 (3 items)                              │
│  • src/plugins/fs/commands.rs:88 (2 items)                      │
│ Full list attached.                                             │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Behavior

- **Thought cards** stream tokens as they arrive (60 fps). Partial flushes on sentence boundary or every 80 ms.
- **Tool-call cards** show tool name, typed args (pretty-printed, collapsed by default), latency, and — once the result arrives — the summary. Expand for full payload (via BlobRef fetch).
- **Permission cards** appear inline when the agent asks for a capability not yet granted. Clicking a button resolves the `oneshot` channel blocking the turn. The three buttons map to `Deny / Grant(once) / Grant(persistent)`.
- **Jump-to-source**: cards with file references (e.g., `src/main.rs:42`) are clickable; the FS plugin opens the file in a preview pane.
- **Replay**: past turns are scrollable, searchable, and re-runnable. A reasoning trace navigates like a stack trace.
- **Stop button**: always visible during a turn in the panel header. Flips the cancel token; the loop unwinds cleanly.
- **Virtualization**: cards outside the viewport + N are virtualized; the panel stays 60 fps with thousands of steps.
- **ARIA**: the panel is an ARIA live region; cards have `role="article"` with labeled headers for screen readers.

### 3.3 Cost badge

Top-right corner of the panel, always visible:

```
┌──────────────────────┐
│ 12.4k in · 3.1k out  │
│       $0.083 · 4.2s  │
└──────────────────────┘
```

Updates on every `agent.cost` sample (policy `CoalesceLatest`). Click to expand into a per-tool breakdown.

---

## 4. Command Palette & Keybindings

### 4.1 Palette

- Universal (`Ctrl/Cmd+K`) front for every user-invokable action.
- Plugins contribute entries via `ModuleManifest::ui_contributions.palette_entries`.
- Fuzzy search, recency-weighted, contextual (hides inapplicable entries when no tab is open).

### 4.2 Keybindings as themes

- Vim-like, Emacs-like, VSCode-like bundles ship stock.
- Each bundle is a **typed object** in the Settings UI with live conflict detection; **there is no `keybindings.json`**.
- Every keybinding maps to a palette entry; there is no keybinding without a corresponding command.
- Custom bundles can be created from the UI and exported (typed schema, imported through a diff view — §1).

### 4.3 Chords

`Ctrl+K Ctrl+S` style chords are supported; the UI shows a chord-in-progress overlay at the bottom of the window.

---

## 5. Accessibility & Theming

- **Contrast**: all shipped themes pass WCAG AA on foreground/background and on interactive elements.
- **Screen reader**: reasoning panel is ARIA-live; tab list uses `role="tablist"`; cards have semantic headers.
- **Reduced motion**: respects OS setting. Disables card entry animations, token streaming easing, and tab-drag inertia.
- **Keyboard-only navigation**: every interactive control is reachable via `Tab`; focus rings are visible and themeable.
- **Themes** are CSS custom-property bundles. The Theme Editor in Settings lets users tweak tokens live and export a shareable bundle — imported through the typed diff-view pathway (§1).

### 5.1 Theme token example (JSON, import/export format only — user never edits it)

```json
{
  "name": "Midnight",
  "tokens": {
    "color.bg.canvas":      "#0b0e14",
    "color.bg.panel":       "#11161f",
    "color.fg.primary":     "#d9e0ec",
    "color.fg.muted":       "#8894a8",
    "color.accent.primary": "#7aa2f7",
    "color.accent.danger":  "#f7768e",
    "font.mono":            "JetBrains Mono, ui-monospace",
    "font.size.ui":         "13px",
    "font.size.term":       "14px",
    "radius.card":          "8px"
  }
}
```

---

## 6. Performance Posture

- **Terminal scrollback**: 60 fps at 100k lines. XTerm.js WebGL addon is non-optional.
- **Reasoning stream**: cards virtualized beyond viewport + N. Token updates batched per animation frame.
- **Cold start target**: interactive terminal in **< 400 ms** on a modern laptop (measured in S8 on CI against a reference machine). Plugins lazy-load post-interactive.
- **IPC budget**: 95th percentile command round-trip under 5 ms. Event throughput 10k events/s without UI jank.
- **Memory**: idle app with one tab, agent panel, and default plugins under 200 MiB RSS.

These are the numbers CI in Sprint 8 will enforce via headless tauri-driver benchmarks.

---

## 7. The Onboarding Flow (first run)

First-run UX is critical for Zero-JSON credibility. The flow:

1. **Welcome screen** — one sentence: "Rustacle is an agentic terminal. Let's set up one local or cloud model."
2. **Auto-discovery** — the app probes `localhost:11434`, `localhost:1234`, etc. If anything answers, one-click "Use Ollama" / "Use LM Studio" cards appear.
3. **Or pick a cloud provider** — OpenAI / Anthropic cards. Clicking opens a guided form: API key (stored in keyring), model pick, temperature default.
4. **Grant initial permissions** — the FS plugin asks for read access to the user's home dir (UI shows exactly the path). Can be narrowed later.
5. **Open a tab** — one terminal tab is auto-opened in the user's home dir.
6. **"Try asking something"** — placeholder suggestion in the chat input.

Everything after this is discoverable from the Settings UI and the command palette. No docs required to reach useful work.

---
*Related: [README](./README.md) · [agent_reasoning](./agent_reasoning.md) · [architecture](./architecture.md) · [project_structure](./project_structure.md) · [ADR-0001](./adr/0001-ui-framework.md)*
