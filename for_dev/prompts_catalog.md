# Prompts Catalog

> Every system-level prompt fragment the agent plugin emits, as compiled constants or user-loaded extensions. This file is the **single source of truth**; the Rust constants in `crates/rustacle-agent/src/prompt/layers.rs` and the registry in `crates/rustacle-agent/src/prompt/registry.rs` must match.

Inspired by `refs/cc-src/constants/prompts.ts::getSystemPrompt` — dynamic composition of static prefixes with environment info, tool descriptions, reminders, and MCP metadata. Rustacle extends this with a **tagged prompt registry** supporting roles, modes, skills, and user extensions.

---

## Architecture

### Two Assembly Paths

1. **`assemble_prompt(ctx)`** (v1) — hardcoded 21-layer sequence, backward-compatible
2. **`assemble_prompt_v2(ctx, registry)`** (v2) — registry-based, filters by role/mode/skills, sorts by priority

Both produce deterministic output for identical inputs (golden-tested via `insta`).

### Prompt Registry

`PromptRegistry` (`crates/rustacle-agent/src/prompt/registry.rs`) indexes prompts by id in a `BTreeMap` (deterministic iteration). Each entry has:

- **id**: unique identifier (e.g., `section-safety`)
- **type**: `Section | Role | Mode | Tool | Skill | Agent`
- **tags**: searchable keywords
- **audience**: `[all]` or specific roles
- **priority**: integer for assembly order (lower = earlier)
- **requires/excludes**: dependency and conflict declarations

### Tagged Markdown Format (for user extensions)

```markdown
---
id: "my-custom-skill"
name: "My Custom Skill"
description: "Custom instructions for my workflow"
type: skill
tags: [custom, workflow]
requires: [section-identity]
excludes: []
audience: [all]
priority: 1500
---

(prompt content in markdown)
```

User prompts are loaded from:
1. `~/.rustacle/prompts/` (global)
2. `.rustacle/prompts/` (project-local)

User entries with the same `id` as a built-in override it.

---

## Built-in Sections

All files in `crates/rustacle-agent/src/prompt/text/`:

### Core Sections (always included)

| ID | File | Priority | Purpose |
|----|------|----------|---------|
| `section-identity` | `section_identity.txt` | 100 | Who the agent is, what it can do |
| `section-system` | `section_system.txt` | 200 | UI/tool pipeline rules, prompt injection detection |
| `section-doing-tasks` | `section_doing_tasks.txt` | 400 | Coding discipline, no gold-plating, faithful outcomes |
| `section-safety` | `section_safety.txt` | 500 | Credentials, destructive action gates, capabilities |
| `section-cyber-boundary` | `section_cyber_boundary.txt` | 510 | Security testing vs harmful activities boundary |
| `section-risk-taxonomy` | `section_risk_taxonomy.txt` | 550 | Rich risk categorization: destructive/hard-to-reverse/shared-state |
| `section-actions` | `section_actions.txt` | 600 | Reversibility analysis, blast radius, measure-twice |
| `section-tool-preference` | `section_tool_preference.txt` | 650 | ALWAYS/NEVER tool hierarchy (fs_read > cat, etc.) |
| `section-tools` | `section_tools.txt` | 700 | Parallel dispatch, tool result persistence |
| `section-files` | `section_files.txt` | 800 | Read-before-edit, path canonicalization |
| `section-bash-safety` | `section_bash_safety.txt` | 850 | Git safety protocol, sleep discipline, chaining rules |
| `section-shell` | `section_shell.txt` | 900 | Tab targeting, absolute paths, long-running commands |
| `section-tone` | `section_tone.txt` | 1000 | Linkification, no emoji, no colon before tool calls |
| `section-output` | `section_output.txt` | 1100 | Conciseness enforcement, inverted pyramid |
| `section-loop-avoidance` | `section_loop_avoidance.txt` | 1850 | Same error twice → stop, diagnose, different approach |
| `section-result-persistence` | `section_result_persistence.txt` | 1860 | Write down important info before results are cleared |

### Role Overlays (one active per turn)

| ID | File | Audience | Focus |
|----|------|----------|-------|
| `role-developer` | `role_developer.txt` | developer | Technical fluency, "why" > "what" |
| `role-manager` | `role_manager.txt` | manager | Impact/outcomes, effort estimates, business risks |
| `role-blogger` | `role_blogger.txt` | blogger | Clear explanations, examples, narrative structure |
| `role-analyst` | `role_analyst.txt` | analyst | Metrics, trade-offs, structured formats |
| `role-devops` | `role_devops.txt` | devops | Idempotency, CI/CD, operational safety |
| `role-designer` | `role_designer.txt` | designer | UX impact, accessibility, interaction states |
| `role-student` | `role_student.txt` | student | Educational, step-by-step, concept naming |

Active role is set via `AgentRole` setting (default: `developer`).

### Mode Overlays (one active per turn)

| ID | File | Mode | Purpose |
|----|------|------|---------|
| `mode-plan` | `overlay_plan_mode.txt` | Plan | Read-only analysis, numbered plan output |
| `mode-ask` | `overlay_ask_mode.txt` | Ask | No tools, direct Q&A from knowledge |

Chat mode has no overlay (full tools, default behavior).

### Model Provider Overlays

| File | Provider | Focus |
|------|----------|-------|
| `overlay_openai.txt` | OpenAI | `tools` parameter, parallel fan-out |
| `overlay_anthropic.txt` | Anthropic | `tool_use` blocks, no custom XML |
| `overlay_local.txt` | Local | Low latency, narrow context, JSON-in-text fallback |

### System Reminders

`system_reminders.txt` — appended just before conversation history. Contains behavioral nudges:
- Skip filler, prefer tools, destructive action gate
- Loop avoidance: "same error twice → stop and diagnose"
- Result persistence: "note important info in text"
- Conciseness: "single-sentence summary when done"

---

## Dynamic Per-Turn Sections

These are computed at assembly time, not stored in the registry:

1. **Model profile overlay** — based on `ctx.model_profile.provider`
2. **Environment context** — OS, shell, cwd, tabs, date/timezone
3. **Tool schemas** — filtered by UI-enabled and permission-granted
4. **Project docs** — RUSTACLE.md/CLAUDE.md, 2000 tokens/file, 8000 total
5. **Selected files** — user-pinned, 8 KiB/file, 10 files max
6. **Memory** — top-K entries (default 6), BM25 + recency decay
7. **System reminders** — from `SYSTEM_REMINDERS` constant
8. **Conversation history** — past turns, middle-first trimming
9. **User turn** — current message

---

## Extension Points

### Skills (`type: skill`)

Not active by default. Activated via `/skill <id>` command or `active_skills` in `TurnContext.extra`. When active, injected at their priority position.

### Agents (`type: agent`)

Full subagent profiles. Used for `sub_agent` tool spawning. The agent's `requires` list declares which sections compose its system prompt.

### User Override

Any built-in prompt can be replaced by a user `.md` file with the same `id`. This allows full customization without forking.

---

## Assembly Order (v2, registry-based)

1. Registry-resolved sections (sorted by priority):
   - Sections (audience-matched)
   - Role overlay (role-matched)
   - Mode overlay (mode-matched)
   - Active skills
2. Model profile overlay
3. Environment context
4. Tool schemas
5. Project docs
6. Selected files
7. Memory
8. System reminders
9. Conversation history
10. User turn

---

## Customization Rules

- Every layer is editable via Settings UI (not by editing files).
- Customizations live in the `ModelProfile`, not global state.
- Safety-critical sentences in core sections cannot be removed (diff-based validation).
- Import/export includes prompt customizations but excludes secrets.

---

*Related: [agent_reasoning](./agent_reasoning.md) · [tools_catalog](./tools_catalog.md) · [ui_ux_manifesto](./ui_ux_manifesto.md)*
