# Security Model

> The full security posture in one place. This file is the canonical reference for sandboxing, command execution isolation, user-data protection, agent protection, and protection **from** the agent.

Companion: [`knowledge_base.md` §4](./knowledge_base.md) (threat model tables), [`architecture.md` §4.7 + §7](./architecture.md) (permission broker, cache discipline).

---

## 1. Layered defenses (summary)

Rustacle enforces isolation at five concentric rings:

```
┌────────────────────────────────────────────────────────────────┐
│ R5 · OS-level: signed binary, OS sandboxing, keyring           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ R4 · Host process: tracing+redaction, trace IDs, panic   │  │
│  │  ┌───────────────────────────────────────────────────┐   │  │
│  │  │ R3 · Permission Broker: capability-gated I/O      │   │  │
│  │  │  ┌─────────────────────────────────────────────┐  │   │  │
│  │  │  │ R2 · WASM Sandbox: wasmtime, fuel, memlimit │  │   │  │
│  │  │  │  ┌───────────────────────────────────────┐  │  │   │  │
│  │  │  │  │ R1 · Plugin logic                     │  │  │   │  │
│  │  │  │  └───────────────────────────────────────┘  │  │   │  │
│  │  │  └─────────────────────────────────────────────┘  │   │  │
│  │  └───────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

A compromise at any ring still has to breach the next. The agent is inside R1; user secrets live in R5.

---

## 2. The three protection goals

### 2.1 Protecting user data **from** the agent

The agent is treated as **untrusted code with user-like privileges**. Assumptions:

- The agent may be prompt-injected by any content it reads (web pages, file contents, tool output).
- The agent may produce tool calls the user did not intend.
- The agent may attempt to exfiltrate secrets via "benign" tool calls.

Mitigations:

1. **No ambient authority.** The agent is a WASM plugin. It cannot open files, sockets, or processes — only call host functions that go through the Permission Broker.
2. **Secrets never enter prompts.** The prompt assembler has no read access to secrets. `ModelProfile` holds a *key name* that the host resolves at request time; the plaintext lives in the OS keyring and never touches the agent's memory space.
3. **Defense-in-depth on outgoing LLM requests.** The LLM router strips any byte sequence that matches a known secret value (from the keyring) before sending a request. A compromised agent trying to stuff a secret into a user-visible prompt field still cannot exfiltrate it.
4. **Destructive-action gate.** `bash` tool refuses destructive patterns (`rm -rf`, `git reset --hard`, `drop table`, …) unless the call carries a model-provided justification AND the user clicks through a permission card. See [`tools_catalog.md` §6](./tools_catalog.md).
5. **FS scopes are user-defined.** The agent sees only paths the user explicitly granted. Canonicalization + prefix match with segment boundary (`/home/k` ≠ `/home/kate`). TOCTOU mitigated via `openat`-style APIs.
6. **Visible tool call before effect.** Every `ToolCall` step emits on the event bus *before* execution, rendered as a card in the Agent Panel. User has a Stop button (< 100 ms cancel).
7. **Network capabilities are host-scoped.** A plugin asking for `Net { hosts: ["api.openai.com"] }` cannot fetch `evil.com` even if prompt-injected.

### 2.2 Protecting **the agent** (and the user's session)

Threats: a malicious plugin, a malicious MCP server, a compromised update.

Mitigations:

1. **Signed plugins.** Ed25519 signature verified against `keys/trusted_plugin_keys.toml` on every load. An unsigned or altered `.wasm` is refused with a visible reason.
2. **WASM hardening.** `wasmtime` Store per instance with **fuel metering** (runaway loop → trap), **memory limit** (default 64 MiB, per-plugin override), **no WASI by default** (custom interface only), narrow host imports.
3. **MCP isolation.** Each MCP server runs in its own OS subprocess (see [`mcp_and_models.md` §4](./mcp_and_models.md)), inheriting only explicit env, scoped to user-granted capabilities. MCP tools pass the same permission broker as native tools.
4. **Update signing.** Tauri updater manifests signed with a key distinct from plugin-signing keys. Compromise of one does not compromise the other.
5. **Providers are user-pinned.** TLS + hostname pinning in `rustacle-llm-*` prevents opportunistic hijacking of traffic to the configured provider.
6. **No auto-load.** The kernel never loads a plugin the user did not explicitly accept in Settings. Drop-in `.wasm` requires an explicit "Install" click.

### 2.3 Protecting the user **from themselves** (safe defaults)

Threats: user accidentally grants over-broad permission, accidentally imports a hostile settings bundle, accidentally runs a destructive command.

Mitigations:

1. **Least privilege by default.** Fresh install grants only FS read on the home directory (narrowed further on request).
2. **One-click narrowing, no JSON.** Every capability shown in Settings has "Narrow" / "Revoke" buttons.
3. **Import preview.** Settings import opens a **diff UI** showing every changed field and every added capability. No silent application.
4. **Destructive Bash guard.** Even on an "always allow shell" grant, destructive patterns still prompt per-call.
5. **Telemetry off by default.** OTLP, Sentry, any outbound telemetry is opt-in per-endpoint, with the endpoint URL shown in Settings.

---

## 3. Command execution isolation

This is the surface most likely to be abused. Details of how Rustacle runs a shell command safely.

### 3.1 Who runs it

The agent plugin **cannot spawn processes**. The `bash` tool inside the agent plugin is a **thin proxy**: it validates, packages the request, sends it as a kernel command to `plugins/terminal`, and awaits streamed output via the event bus.

```
agent (wasm)  ──(kernel cmd "terminal.exec")──►  terminal (native)
                                                        │
                                                        ▼
                                                 portable-pty spawn
                                                        │
  ◄──(terminal.output stream, terminal.exit event)──────┘
```

`plugins/terminal` is the **only** component with `Pty` capability and is native precisely because it needs OS process spawn. Its code is kept minimal and reviewed carefully.

### 3.2 What the terminal plugin enforces

1. **Bounded PTY env.** The spawned shell inherits only a whitelisted env (`PATH`, `HOME`, `TERM`, `LANG`, `USER`, `SHELL`, `PWD`). Secret-named env vars (anything in `keys`, `*_TOKEN`, `*_KEY`, `*_SECRET`, … configurable) are stripped.
2. **Working dir is the target tab's cwd.** The agent cannot pass an arbitrary `cwd`; it picks a tab, and the tab's cwd is used.
3. **No elevated privilege.** No `sudo`, no setuid probing. If a command needs sudo, the user types their password interactively in the terminal.
4. **Timeout enforced in the kernel.** `bash.timeout_ms` is clamped (max 10 min) and enforced by the kernel, not by the shell.
5. **Cancel-safe.** Each `exec` holds a `ChildCancel` token; Stop flips it → kernel sends `SIGTERM` → after 3 s grace → `SIGKILL`.
6. **Output streamed, never buffered indefinitely.** PTY read goes through `TerminalChunk` events with `DropOldest` policy — a runaway command cannot OOM the host.
7. **Record to SQLite.** Every exec is recorded: command, tab, cwd, exit code, duration, output bytes, initiator (user / agent / tool). Audit log visible in Settings → Audit.

### 3.3 Per-OS specifics

| OS | PTY backend | Notes |
|---|---|---|
| Linux | `portable-pty` → `openpty` | Signals `SIGTERM`, `SIGKILL`. `PATH` respects user default shell. |
| macOS | `portable-pty` → `openpty` | Same as Linux; notarized binary retains Gatekeeper exceptions only for updater. |
| Windows | `portable-pty` → **ConPTY** (Windows 10 1809+) | Signals emulated via `ClosePseudoConsole`; fallback legacy PTY disabled. |

Cross-platform specifics in [`cross_platform.md`](./cross_platform.md).

### 3.4 Optional OS sandbox for spawned commands (future)

Stretch goal (post-1.0): run spawned shells inside an OS sandbox:

- Linux: `landlock` + `seccomp` with a narrow policy matching the tab's FS scope.
- macOS: `sandbox_init` with a custom profile.
- Windows: `AppContainer` / `Job Object` with UI restrictions.

Tracked in ADR-0004 (to be filed pre-Sprint-8).

---

## 4. Agent prompt-injection defense

Prompt injection is inevitable: any content the agent reads may try to steer it. Defenses:

1. **User visibility.** Because every tool call appears as a card before it runs, the user is in the loop. Stop button is always one keystroke away.
2. **Capability gating.** Even an injected instruction cannot execute a tool the user has not granted.
3. **Outbound secret scrubbing** (2.1 #3).
4. **No `curl | sh`-like patterns.** The `bash` tool's destructive guard forbids common shell-pipe-exec patterns when they involve network fetches (configurable).
5. **Tool outputs carry a `TAINTED` marker.** Text content retrieved from external sources (web, fetched files from user-untrusted paths) is wrapped by the tool result in `<untrusted-content>…</untrusted-content>` tags in subsequent turns, reminding the model not to follow instructions inside.
6. **Reasoning replay.** Users can replay any turn; suspicious traces can be audited after the fact.

---

## 5. Data-at-rest protection

- **Secrets**: OS keyring only. Never in SQLite, never in files, never in logs.
- **Settings DB**: SQLite, WAL mode. File permissions `0600` on Unix; Windows ACL restricted to the current user.
- **Reasoning traces**: in SQLite, subject to UI-configured retention (default 30 days). "Delete all my data" button wipes DB + blob store + memory.
- **Blob store**: under the app data dir, same permissions. Blobs GC'd on startup per retention.
- **Crash reports / telemetry**: off by default. When enabled, secrets redacted by the `tracing` redactor layer before any export.

---

## 6. Data-in-motion protection

- **LLM provider calls**: HTTPS with rustls, hostname pinning. Local providers (Ollama) are HTTP on `localhost` only — the loopback case is whitelisted; anything else must be HTTPS.
- **MCP servers**: subprocess stdio by default (no network surface); HTTP+SSE mode requires explicit `Net` capability for the host/port.
- **Updater**: HTTPS + signed manifest.
- **Telemetry**: HTTPS only, user-provided endpoint.

---

## 7. Audit trail

Everything an agent does is logged:

- **Reasoning steps** — `reasoning_steps` table, replay UI.
- **Tool calls** — included in reasoning steps.
- **Permission grants / revocations** — `permission_audit` table.
- **Shell exec** — `exec_audit` table with initiator and duration.
- **Settings changes** — `settings_audit` table with before/after.
- **Plugin load/unload** — `plugin_audit` table.

All seven streams viewable in Settings → Audit. Exportable as typed JSON (secrets excluded) for external analysis.

---

## 8. Key management

| Key | Purpose | Storage | Rotation |
|---|---|---|---|
| Plugin-signing pubkeys | Verify `.wasm` signatures | `keys/trusted_plugin_keys.toml` (checked in) | Manual; ADR-0003 describes process |
| Plugin-signing privkey | Sign team plugins | Developer HSM / password manager | Per-release |
| Updater pubkey | Verify update manifests | Embedded in binary at build time | Per major release |
| Updater privkey | Sign updates | CI secret | Annual |
| User API keys | Provider auth | OS keyring | User-controlled |

**Key separation** is mandatory: compromise of one key class must not affect another. See `tech_stack_2026.md` §10.

---

## 9. Security review cadence

- **Every sprint**: review new capability declarations, new host fns, new threat-surface entries.
- **Sprint 8**: external security review (or internal red-team). P0 / P1 findings block release.
- **Post-1.0**: rolling reviews per major feature; dependency advisory scan in CI (`cargo audit` + `cargo deny check`).

---

## 10. Incident response

- Security issues reported privately (email, to be published in SECURITY.md pre-release).
- Fixes land as patch releases; updater pushes automatically to opted-in users.
- Affected plugin signing keys are revoked by publishing a new `trusted_plugin_keys.toml` via the updater.
- Post-mortem published after users are patched.

---
*Related: [README](./README.md) · [concept](./concept.md) · [architecture](./architecture.md) · [knowledge_base](./knowledge_base.md) · [cross_platform](./cross_platform.md) · [mcp_and_models](./mcp_and_models.md) · [observability](./observability.md)*
