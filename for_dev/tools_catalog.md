# Tools Catalog

> The authoritative list of every stock tool shipping with `plugins/agent`, including its name, description (as the LLM will see it), input schema (JSON Schema), concurrency class, required capabilities, and behavioral notes.

All tools implement the `Tool` trait defined in `agent_reasoning.md` §4.1. Patterns are drawn from `refs/cc-src/tools/*` (the `buildTool` factory at `Tool.ts:783`).

---

## Design principles

1. **The description is the tool's prompt.** The LLM will decide whether to call a tool based mostly on its `description`. Write it like a help page, not a docstring.
2. **Input schemas are strict.** `additionalProperties: false`, required fields listed, types narrow. Loose schemas lead to hallucinated arguments.
3. **Validate in pure Rust before permission check.** Rejecting an obviously-bad call early saves a round-trip and a scary permission dialog.
4. **Concurrent iff read-only and idempotent.** If running two copies at once could interleave or corrupt, it's `Serialized`.
5. **Summaries are short.** `ToolResult::summary` is what the UI shows in the card. Heavy payloads go to `BlobRef`. See `agent_reasoning.md` §2.1.
6. **Errors are observations, not crashes.** A tool returning `Err` becomes a `ToolResult { ok: false, summary: <error> }` — the model sees it and reacts. Only panics/traps become `ReasoningStep::Error`.
7. **Every tool has a golden test.** One fixture per tool in `plugins/agent/tests/tools/`.

---

## 1. `fs_read` — read a file

**Pattern**: `refs/cc-src/tools/FileReadTool/FileReadTool.ts`.

**Description** (sent to the model):
> Read the contents of a file on the user's filesystem. Supports text files, binary files (returned as a short summary), and image files (returned as a description + dimensions). For very large files, reads are clipped; pass `offset` and `limit` to page through.

**Concurrency**: `Concurrent`.

**Required capabilities**: `Fs(scope: <path>, mode: Read)`.

**Input schema**:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["path"],
  "properties": {
    "path": { "type": "string", "description": "Absolute path." },
    "offset": { "type": "integer", "minimum": 0, "description": "Line offset." },
    "limit":  { "type": "integer", "minimum": 1, "maximum": 5000, "description": "Max lines to read." }
  }
}
```

**Validation (Rust)**:

```rust
fn validate(&self, args: &Value) -> Result<(), ToolError> {
    let path = args.get("path").and_then(Value::as_str)
        .ok_or(ToolError::InvalidInput("path is required".into()))?;
    if !Path::new(path).is_absolute() {
        return Err(ToolError::InvalidInput("path must be absolute".into()));
    }
    Ok(())
}
```

**Behavior**:
- Canonicalize → check scope → open.
- If binary (detected by null-byte scan): return a `summary` like `"binary, 1.2 MiB, sha256=..."`, no payload_ref.
- If image: return dimensions and a short description, no payload_ref.
- Text: head + tail of the file if > 5000 lines; full content under 500 lines; otherwise windowed by `offset/limit`.

**Golden test**: `tests/tools/fs_read/binary.snap`, `text_small.snap`, `text_large_windowed.snap`.

---

## 2. `fs_write` — write / create a file

**Pattern**: `refs/cc-src/tools/FileWriteTool/FileWriteTool.ts`.

**Description**:
> Create a new file or overwrite an existing one with the given contents. For modifying parts of an existing file, prefer `fs_edit`. For files over 1 MiB, this tool refuses; use multiple edits instead.

**Concurrency**: `Serialized`.

**Required capabilities**: `Fs(scope: <parent dir>, mode: Write)`.

**Input schema**:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["path", "content"],
  "properties": {
    "path":    { "type": "string", "description": "Absolute path." },
    "content": { "type": "string", "description": "New file contents (UTF-8)." }
  }
}
```

**Behavior**:
- Reject if `content` > 1 MiB.
- Reject binary writes (null bytes in `content`).
- Parent directory must exist (don't silently mkdir — ask the agent to do it with a shell tool).
- Preserve existing line endings if overwriting.

**Summary format**: `"wrote 1823 lines (48.2 KiB) to src/main.rs"`.

---

## 3. `fs_edit` — string-replace edit

**Pattern**: `refs/cc-src/tools/FileEditTool/FileEditTool.ts`.

**Description**:
> Replace an exact string in a file with a new string. The old string must be unique in the file; if it isn't, the call fails and you should include more surrounding context to make it unique. Indentation must be preserved exactly.

**Concurrency**: `Serialized`.

**Required capabilities**: `Fs(scope: <file>, mode: Write)`.

**Input schema**:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["path", "old_string", "new_string"],
  "properties": {
    "path":        { "type": "string" },
    "old_string":  { "type": "string", "minLength": 1 },
    "new_string":  { "type": "string" },
    "replace_all": { "type": "boolean", "default": false }
  }
}
```

**Behavior**:
- Read file, enforce UTF-8.
- If `replace_all=false`: reject with `"old_string appears N times"` if N ≠ 1.
- If `replace_all=true`: replace every occurrence (useful for renames).
- Compute a diff (using `similar` crate); `summary` is the diff stat `"+3 -1 lines in src/lib.rs"`.
- `payload_ref` holds the full unified diff for the UI to expand.

---

## 4. `grep` — pattern search

**Pattern**: `refs/cc-src/tools/GrepTool/GrepTool.ts`.

**Description**:
> Search files for a regex pattern using ripgrep semantics. Much faster than shelling out to grep. Supports globs, file types, and context lines. Use this before resorting to `bash` for searching.

**Concurrency**: `Concurrent`.

**Required capabilities**: `Fs(scope: <path>, mode: Read)`.

**Input schema**:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["pattern"],
  "properties": {
    "pattern": { "type": "string", "description": "Ripgrep regex." },
    "path":    { "type": "string", "description": "Directory or file to search. Defaults to cwd." },
    "glob":    { "type": "string", "description": "Glob filter, e.g. *.rs" },
    "type":    { "type": "string", "description": "rg file type, e.g. rust, python." },
    "context": { "type": "integer", "minimum": 0, "maximum": 10, "default": 0 },
    "case_insensitive": { "type": "boolean", "default": false },
    "max_matches": { "type": "integer", "minimum": 1, "maximum": 500, "default": 100 }
  }
}
```

**Backend**: the `grep` crate (ripgrep as a library), **never** shelling out. No shell escaping, no injection.

**Summary format**: `"47 matches in 12 files"`. Payload_ref holds the full match list.

---

## 5. `glob` — file pattern match

**Pattern**: `refs/cc-src/tools/GlobTool/GlobTool.ts`.

**Description**:
> Find files matching a glob pattern. Returns paths sorted by modification time, newest first. Use this to locate files by name; use `grep` to search contents.

**Concurrency**: `Concurrent`.

**Required capabilities**: `Fs(scope: <path>, mode: Read)`.

**Input schema**:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["pattern"],
  "properties": {
    "pattern": { "type": "string", "description": "Glob, e.g. src/**/*.rs" },
    "path":    { "type": "string", "description": "Root; defaults to cwd." },
    "limit":   { "type": "integer", "minimum": 1, "maximum": 1000, "default": 250 }
  }
}
```

**Summary**: `"found 42 files (showing 42)"`.

---

## 6. `bash` — run a shell command

**Pattern**: `refs/cc-src/tools/BashTool/BashTool.tsx` — note the 6-layer validation (bash permissions, security, readonly, sed, path, mode).

**Description**:
> Run a shell command in a terminal tab. You can redirect to a specific tab via `tab_target`. Long-running commands should be split into smaller steps. For file search, prefer `grep`/`glob`. For file reads, prefer `fs_read`. For file edits, prefer `fs_edit`. Use `bash` for anything else: builds, tests, git, etc.

**Concurrency**: `Serialized`.

**Required capabilities**: `Pty` (native-only today).

**Input schema**:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["command"],
  "properties": {
    "command":    { "type": "string", "description": "The command to run." },
    "tab_target": { "type": ["string", "null"], "description": "TabId; defaults to active tab." },
    "timeout_ms": { "type": "integer", "minimum": 100, "maximum": 600000, "default": 60000 },
    "run_in_background": { "type": "boolean", "default": false }
  }
}
```

**Validation** (six layers, adapted from cc-src `BashTool`):

1. **Permissions check** — capability broker (separate from the Rust `validate()` step).
2. **Destructive-command warning** — pattern-match `rm -rf`, `git reset --hard`, `git push --force`, `drop table`, `:wq!`, `dd if=`. On match, force the model to include a human-readable justification in the call args, otherwise reject.
3. **Read-only-mode enforcement** — if the user toggled "read-only mode" in Settings, reject any command matching a write-pattern list.
4. **Sed in-place** — reject `sed -i` (prefer `fs_edit`).
5. **Path check** — reject absolute paths outside the tab's `Fs` scope grants (defense-in-depth; the shell will also enforce).
6. **Mode check** — reject interactive commands (`vim`, `less`, `htop`, …) unless `run_in_background=true` or the user explicitly enabled interactive-shell mode.

**Execution**: delegated to `plugins/terminal` via a kernel-mediated command (the agent plugin is WASM and cannot spawn processes). Output streams arrive as `terminal.output` events; the tool awaits until exit or timeout.

**Summary format**: `"exit 0 in 2.3s, 1843 bytes output"`.

---

## 7. `sub_agent` — spawn a child harness

**Pattern**: `refs/cc-src/tools/AgentTool/runAgent.ts`.

**Description**:
> Delegate a self-contained task to a fresh agent instance. The child runs with its own conversation history and tool set but the same permissions. Use this to parallelize research, isolate context, or run a large task without polluting the main conversation.

**Concurrency**: `Serialized` by default. (Configurable per-profile to allow parallel children with a bounded pool.)

**Required capabilities**: `LlmProvider`.

**Input schema**:

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["task"],
  "properties": {
    "task": { "type": "string", "description": "A self-contained instruction." },
    "tool_allowlist": { "type": "array", "items": { "type": "string" } },
    "model_profile":  { "type": ["string", "null"], "description": "Override model profile." },
    "budget_turns":   { "type": "integer", "minimum": 1, "maximum": 20, "default": 8 }
  }
}
```

**Behavior**:
- Spawns a new harness inside the same plugin instance.
- Child `ReasoningStep`s set `parent_id` to the parent step; the UI renders a collapsible subtree.
- Child budget is enforced independently; exceeding it ends the child with an `Error` step, not the parent.
- Returns a final summary `Answer` to the parent loop.

---

## 8. (Future / stretch) `sqlx_query`, `http_fetch`, `python_exec`

Listed here so design discussions have a shared target. Not shipping in 1.0.

| Tool | Why later | Notes |
|---|---|---|
| `sqlx_query` | Needs per-database scoping model | Could live as a separate plugin. |
| `http_fetch` | Overlap with `Net` capability design | Wait for a threat-model review of outbound-HTTP-as-tool. |
| `python_exec` | Needs sandboxed python runtime | `pyodide-in-wasm` is the likely path. |

---

## 9. User-defined tools (skills)

`plugins/skills` loads user-authored "skills" from a configurable directory. A skill is a small declarative bundle:

```
my-skill/
├── skill.toml
└── handler.wasm       # optional; for native-backed skills, otherwise uses built-in runners
```

`skill.toml` maps 1:1 to the `Tool` trait (name, description, input schema, required capabilities, concurrency). Users manage skills from the Settings UI (browse bundled directory, enable/disable, view audit log of calls). Skills are sandboxed exactly like core plugins.

---
*Related: [README](./README.md) · [agent_reasoning](./agent_reasoning.md) · [prompts_catalog](./prompts_catalog.md) · [architecture](./architecture.md)*
