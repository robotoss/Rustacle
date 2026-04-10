# UI Simplicity Principles

> How Rustacle looks and feels different from existing AI CLIs and IDE chat panels. What "simple" actually means in this project — not fewer features, but fewer decisions the user must make to get to useful work.

Companion: [`ui_ux_manifesto.md`](./ui_ux_manifesto.md) (hard rules, wireframes). This file is the philosophy; the manifesto is the law.

---

## 1. The thesis

Existing AI tools force the user to choose between two bad modes:

- **Chat-only** tools (Claude Code CLI in its default form, various terminal chats) — powerful, but the user is blind to what the agent is doing. Every new feature adds a new flag, a new JSON field, a new mental model.
- **IDE-sidebar** tools — beautiful UI, but agent is glued to a file editor and cannot do real terminal work across multiple shells. Extension is gated behind an IDE's plugin system.

Rustacle's bet: you can have **visible agent reasoning**, **terminal-native work**, and **zero configuration pain** at the same time, if you make a few principled choices and hold the line on them.

---

## 2. The five simplicity principles

### 2.1 One screen, one truth

There is **one** way to see what the agent is doing: the Agent Panel. It streams every reasoning step as a card. There is no second "log view", no `--verbose` flag, no "developer mode" toggle that reveals hidden info. What the panel shows is everything.

### 2.2 Every setting is a control

If a feature has a setting, that setting is a typed control in Settings UI. Discovering it is a matter of scrolling through a page, not grepping a man page or hunting through a JSON file. See [`ui_ux_manifesto.md` §1](./ui_ux_manifesto.md).

### 2.3 The palette is the help

The command palette (`Ctrl+K`) fronts every user-invokable action, across every plugin. Learning Rustacle means learning `Ctrl+K` — there is no second level of "power-user commands" that lives elsewhere.

### 2.4 Stop is one keystroke

The agent is a powerful thing running on the user's machine. The single most important affordance is the ability to stop it cleanly, in under 100 ms, without losing state. `Ctrl+.` stops the current turn. The Stop button is always visible during a turn. The user is never stuck waiting on an agent they wanted to interrupt.

### 2.5 Onboarding is four screens

First run:

1. Welcome (one sentence).
2. Pick a model (auto-discovered local or a cloud card).
3. Grant initial FS scope.
4. Open a terminal tab with a "try asking" placeholder.

That's it. No tour, no multi-step wizard, no "create a workspace". Four screens and the user is productive.

---

## 3. How this distinguishes Rustacle (concrete comparisons)

### 3.1 vs "Claude Code"-class CLIs

| Axis | Typical CLI | Rustacle |
|---|---|---|
| Seeing what the agent is thinking | Streamed prose; often hidden by default | First-class typed `Thought` cards, always visible |
| Seeing what the agent is about to do | Prose says "I'll run X" then runs it | `ToolCall` card emitted **before** execution; Stop button kills it |
| Seeing the cost | Sometimes shown at turn end | Live badge; per-tool attribution |
| Configuring a model | Edit `~/.claude/config.json`, restart | Click in Settings, no restart |
| Adding a tool | Write a shell wrapper, register in a config file | Drop a signed WASM plugin, click install |
| Multiple shells | Run multiple CLIs in multiple terms | Tabs + splits in one window, per-tab agent context |
| Permissions | Global allow-lists in config | Per-capability broker, per-plugin grants, audit trail |
| Recovery from a bad tool call | Kill the process, lose history | Stop button, reasoning trace replayable |
| Switching local ↔ cloud model | Re-run with env vars | One dropdown in the chat input |
| Secret handling | Env vars, files | OS keyring, redacted in logs |

### 3.2 vs IDE chat panels

| Axis | IDE sidebar | Rustacle |
|---|---|---|
| Terminal work | One embedded terminal, often limited | Full multi-tab terminal, native PTY |
| Agent scope | Usually scoped to current file/project | Arbitrary FS scope (user-granted), multiple projects |
| Daily-driver viability | You still open a real terminal | Rustacle is the terminal |
| Plugin story | IDE's plugin system | Signed WASM sandbox + MCP |
| Reasoning visibility | Varies by product | First-class, uniform |
| Offline / local models | Rare, often an afterthought | Local-first; cloud is optional |

### 3.3 Versus "just use Claude in a browser"

Rustacle runs on your machine, sees your filesystem, runs your shells, streams tokens without a web tab. Nothing crosses the network by default unless you picked a cloud model.

---

## 4. The simplicity budget (what we refuse to add)

These are the pressures we anticipate and the reasons we resist each.

| Pressure | Rejected because |
|---|---|
| "Add a preferences.json so power users can edit directly" | Breaks Zero-JSON. Every preference has a UI control, period. |
| "Add a CLI mode that does the same thing as the GUI" | Dilutes the one-screen-one-truth principle. GUI is the only surface. |
| "Add a 'quiet' mode that hides reasoning" | Hidden reasoning is the exact thing we exist to fix. |
| "Add a marketplace inside the app" | Plugin install is a file drop + Settings click. A marketplace is a separate product. |
| "Add account sync" | Nothing leaves the machine by default. Sync is a separate, opt-in feature with its own ADR. |
| "Let plugins ship arbitrary JS into the UI" | Violates the WASM sandbox principle. Plugins contribute typed UI via manifest, nothing more. |
| "Let skills run arbitrary native code" | Same reason. Skills are WASM or declarative. |
| "Auto-approve permissions the user already granted once globally" | Permission cache is per-plugin, per-capability; global bypass defeats the broker. |

Every "no" above is a door we're holding shut deliberately. When a user asks for one of these, the answer is "here's the principle, here's the alternative we do support."

---

## 5. Visual simplicity rules

### 5.1 Hierarchy

- **Three zones** on the default layout: terminal (left/center), agent panel (right), title/tab bar (top).
- **No nested sidebars.** No "activity bar". No floating inspector windows.
- **Two font sizes in chrome**: UI 13 px, terminal 14 px. That's it. Users can rescale via accessibility settings.

### 5.2 Motion

- Motion is for information, not decoration. Tokens streaming is motion because it conveys the stream's progress. A button glow because it's hovered is not.
- Respect `prefers-reduced-motion` everywhere. No animations on reduced-motion users.
- Max animation duration 200 ms, ease-out.

### 5.3 Color

- One accent color per theme; everything else is a neutral.
- Red reserved for destructive actions and errors.
- No gradients in interactive elements.

### 5.4 Copy

- **Active voice, present tense.** "Ready" not "The system is ready".
- **No jargon without a definition.** First mention of "capability" or "profile" links to the glossary tooltip.
- **Error messages point at the control**, never at a file.
- **No exclamation points.** No emojis in default UI copy. (Users can enable emojis in themes.)

### 5.5 Empty states

Every empty state has one sentence and one suggested action:

```
┌───────────────────────────────────────────┐
│                                           │
│   No tools enabled yet.                   │
│                                           │
│   [ Open Settings → Tools ]               │
│                                           │
└───────────────────────────────────────────┘
```

---

## 6. What gets cut if it doesn't earn its space

When a feature proposal arrives, we ask:

1. Does it replace an existing surface, or add a new one? **Additions are expensive.**
2. Can it live in a plugin instead of core UI?
3. Does it compete for visual space with the Agent Panel or the terminal?
4. Does it introduce a second way to do something we already have a way to do?
5. Can it be a command-palette entry instead of a dedicated button?

The palette is the dumping ground for features that want a home but don't earn a permanent UI surface. Most things belong there.

---

## 7. Keyboard-first, mouse-friendly

- Every action reachable by keyboard (`Tab`, palette, keybinding).
- Mouse works but is never required.
- Drag is reserved for spatial operations: moving tabs between windows, redirecting tool calls between tabs, resizing splits. Not for sorting lists or triggering actions.

---

## 8. Progressive disclosure

Simple doesn't mean shallow. Advanced features exist — they just aren't in the user's face by default.

- **Default view of a `ToolCallCard`**: one-line summary.
- **Expanded**: full args, output, diff, latency, cost.
- **Right-click**: "Re-run in a new tab", "Copy args", "Explain this tool".

First view: minimal. Every click reveals more. Nothing is hidden — it's just layered.

---

## 9. Failure as a feature

When something goes wrong, the UI is more informative, not less:

- Every error surfaces a trace ID the user can click to copy.
- Every degraded mode (bus lag, plugin suspended, provider retrying) is a visible indicator in chrome, not a silent slowdown.
- "Replay this turn" and "Re-run from here" let users recover without losing context.

The worst thing a UI can do under failure is pretend everything is fine. Rustacle shows the seams clearly — and makes them navigable.

---

## 10. The one-sentence test

If a new screen or feature cannot be described in one sentence a user would understand, it isn't ready to ship. This applies equally to internal engineers ("the reasoning panel streams every agent step as a typed card") and to user-facing labels ("Click to allow this plugin to read the folder /home/k/projects").

---
*Related: [README](./README.md) · [concept](./concept.md) · [ui_ux_manifesto](./ui_ux_manifesto.md) · [agent_reasoning](./agent_reasoning.md)*
