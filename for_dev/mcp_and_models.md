# Models, Providers & MCP

> How Rustacle talks to language models and external tools. How local and cloud providers are configured, discovered, selected, and switched. How MCP servers plug in and are sandboxed.

Companion: [`agent_reasoning.md` §6](./agent_reasoning.md) (LlmProvider trait), [`security.md`](./security.md), [`modularity.md` §1.2](./modularity.md).

---

## 1. Terminology

- **Provider** — an implementation of the `LlmProvider` host trait (see `rustacle-llm/src/provider.rs`). Examples: `rustacle-llm-openai`, `rustacle-llm-anthropic`, `rustacle-llm-local`.
- **Endpoint** — a URL the provider talks to. One provider can cover many endpoints (Ollama, LM Studio, vLLM, llama.cpp-server, a remote OpenAI-compatible proxy — all one provider).
- **Model profile** — a user-configured bundle `(provider, endpoint, model_id, temperature, max_tokens, system_overlay, enabled_tools)`. Multiple profiles per user.
- **Active profile** — the profile the agent uses for the current turn. Can be switched per tab.
- **MCP server** — a [Model Context Protocol](https://modelcontextprotocol.io) server (subprocess or HTTP) exposing tools, resources, and prompts. Launched and managed by `plugins/mcp-client`.

---

## 2. Provider trait (refresher)

```rust
// crates/rustacle-llm/src/provider.rs

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;

    async fn stream(
        &self,
        req: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<BoxStream<'static, Result<ChatDelta, ProviderError>>, ProviderError>;

    /// Optional — probe the endpoint to validate creds/list models.
    async fn health_check(&self) -> Result<HealthReport, ProviderError>;

    /// Optional — list models known to the endpoint (for Settings UI dropdowns).
    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![])
    }
}

pub struct ProviderCapabilities {
    pub tool_use: bool,
    pub vision: bool,
    pub streaming: bool,
    pub max_context: u32,
    pub supports_json_mode: bool,
    pub supports_mcp_tools: bool,     // can the model call tools at all?
}
```

Providers live as **host-side** crates (not WASM) — rationale in `modularity.md` §1.2.

---

## 3. Shipped providers

### 3.1 OpenAI-compatible (`rustacle-llm-openai`)

One provider, many endpoints. Anything speaking the OpenAI `chat/completions` dialect routes through this crate.

**Works with:**

| Endpoint | Default URL | Notes |
|---|---|---|
| OpenAI cloud | `https://api.openai.com/v1` | Keyring `openai_api_key`. |
| Ollama | `http://localhost:11434/v1` | No key. Auto-discovered. |
| LM Studio | `http://localhost:1234/v1` | No key. Auto-discovered. |
| vLLM | user-provided URL | Optional bearer token. |
| llama.cpp-server | `http://localhost:8080/v1` | No key. |
| OpenRouter / Together / Groq / Fireworks / … | user-provided URL | Bearer token from keyring. |

### 3.2 Anthropic (`rustacle-llm-anthropic`)

Dedicated client because tool-use dialect differs. Reads `anthropic_api_key` from keyring.

### 3.3 Local autodiscovery (`rustacle-llm-local`)

A meta-provider that probes standard local ports at startup and creates **one-click model profiles** the user can enable from the first-run screen or Settings:

```
Probing http://localhost:11434 ... Ollama ok, 7 models found
Probing http://localhost:1234  ... LM Studio ok, 2 models
Probing http://localhost:8080  ... llama.cpp-server ok, 1 model
```

Each discovered endpoint becomes a "suggested profile" card in the UI. Click → profile created with sensible defaults.

### 3.4 Future (ADR-required)

- Google Vertex / Gemini — `rustacle-llm-google`.
- AWS Bedrock — `rustacle-llm-bedrock`.
- Local Rust inference (e.g. `mistral.rs`) as an embedded provider.

---

## 4. Configuring a provider (Zero-JSON)

**The user never edits a config file.** Every provider/endpoint/key is a typed control in the Settings UI.

### 4.1 First-run flow

1. "Welcome — pick a model" screen.
2. Auto-discovered local endpoints listed as cards (one-click install).
3. Cloud provider cards (OpenAI / Anthropic) open a form:
   - Endpoint URL (prefilled, editable).
   - API key (paste field; on submit, moved to OS keyring; field clears; shows `••••`).
   - Model dropdown (populated from `list_models()`; fallback to a curated list if the endpoint doesn't support listing).
   - "Test connection" button calls `health_check()` and shows a green/red badge.
4. On save, a `ModelProfile` is created in `rustacle-settings`, pointing at a keyring key (not the plaintext).

### 4.2 Settings UI → Model Profiles

| Field | Control |
|---|---|
| Name | text input |
| Provider | dropdown (auto-populated from the registered providers) |
| Endpoint URL | text input |
| API key | `••••` + "Edit" button (opens a paste dialog) |
| Model ID | dropdown (from `list_models`) |
| Temperature | slider |
| Max tokens | number |
| Max context | readonly (from `ProviderCapabilities`) |
| Tool use | readonly checkbox (from capabilities) |
| Vision | readonly checkbox |
| Persona override | textarea (up to 500 chars) |
| Enabled tools | checkbox grid |
| System prompt overlay | "Edit layers..." button → tabbed editor |

Every field is typed, validated, and persisted through `rustacle-settings`. Secrets go to the keyring; everything else to SQLite.

### 4.3 Switching profiles

- Default active profile declared in Settings.
- Per-tab override: right-click tab → "Use profile: …".
- The current turn's profile is snapshotted in `TurnContext` at start; switching mid-turn takes effect on the **next** turn.

### 4.4 Local model setup walkthrough (Ollama example)

1. User installs Ollama from its site.
2. User runs `ollama pull llama3.2`.
3. User opens Rustacle → Settings → Model Profiles → "+ New".
4. If Ollama is running, it appears in the auto-discovery list as a card; one click creates the profile with `llama3.2` preselected.
5. User clicks "Test connection" → green.
6. User enables desired tools, hits Save.
7. New profile appears in the active-profile dropdown in the chat input.

No JSON. No file edits. No terminal commands (inside Rustacle — Ollama's own CLI is of course outside our scope).

---

## 5. MCP integration

Rustacle is a first-class MCP client. MCP servers plug in as **tool + resource providers** subject to the same permission model as native tools.

### 5.1 Architecture

```
plugins/mcp-client (wasm)
     │
     │ spawns via host fn `process-spawn` (whitelisted for this plugin only)
     ▼
┌──────────────────────┐
│ MCP server subproc   │   one per configured server
│ (stdio or HTTP+SSE)  │
└──────────────────────┘
     │
     │ JSON-RPC 2.0 per MCP spec
     │
     ▼
plugins/mcp-client translates MCP tool defs → Rustacle Tool trait
     │
     │ publishes mcp.tool.available on the bus
     ▼
plugins/agent merges into its ToolDispatchTable for the next turn
```

### 5.2 Server config (Zero-JSON)

Settings → Integrations → MCP Servers → "+ Add":

| Field | Control |
|---|---|
| Name | text |
| Transport | dropdown (`stdio` / `http+sse`) |
| Command (stdio) | text input (command + args) |
| URL (http) | text input |
| Working dir | directory picker |
| Environment | key/value list (secret values go to keyring) |
| Capabilities | permission controls (mirrors native plugin capabilities) |
| Auto-start | checkbox |

No `mcp.json` — **everything** is typed and stored in `rustacle-settings`.

### 5.3 Permission model for MCP

When an MCP server advertises a tool, Rustacle does **not** auto-enable it. The user sees a permission card in the Agent Panel the first time the agent wants to call it:

```
🛡 Permission Ask
MCP server "github-mcp" is offering tool "create_issue".
This tool requires: Net(api.github.com), Secret(github_token).
                                    [ Deny ] [ Allow once ] [ Allow always ]
```

MCP servers cannot bypass the permission broker. A malicious MCP server can only do what the user has explicitly granted.

### 5.4 MCP server lifecycle

- **Start**: `plugins/mcp-client` calls the host `process-spawn` fn (whitelisted only for this plugin) with the configured command.
- **Handshake**: MCP `initialize` request, list tools/resources/prompts.
- **Publish**: each discovered capability published on `mcp.tool.available`, `mcp.resource.available`, `mcp.prompt.available`.
- **Heartbeat**: periodic ping; dead servers are marked failed in the UI.
- **Shutdown**: on app exit or user toggle, `plugins/mcp-client` sends MCP `shutdown` then SIGTERM.
- **Restart**: on config change, the server is cleanly restarted with new settings.

### 5.5 MCP subprocess isolation

MCP servers run in their own OS subprocesses (not inside WASM — we can't control third-party languages). Isolation measures:

- **Stripped environment**: only whitelisted vars, like the shell exec path (see `cross_platform.md` §4).
- **Scoped working dir**: inherited from config, canonicalized, must lie within a user-granted FS scope.
- **No inherited fds** beyond stdio.
- **Stdin/stdout are the only channels** in stdio mode.
- **Audit**: every tool call through an MCP server is logged in `mcp_audit` with args and outcome.

### 5.6 Using MCP tools from the agent

From the agent's perspective, an MCP tool looks **identical** to a native tool: same `Tool` trait, same schema, same concurrency class, same permission check. The only difference is the implementation routes through `plugins/mcp-client` back to the subprocess.

This means every feature that works with native tools works with MCP tools: visible reasoning cards, tab redirection (where meaningful), cost tracking, audit trail, Stop button.

### 5.7 MCP resources and prompts

- **Resources**: MCP resource URIs (e.g., `file://`, `http://`, server-defined schemes) can be pinned by the user (Settings → MCP → Resources → Pin). Pinned resources are injected into the prompt as a selected-file-style block per `prompts_catalog.md` §5.
- **Prompts**: MCP prompts appear in the command palette. Invoking one creates a chat turn pre-filled with the prompt's template.

---

## 6. Choosing models: UX rules

- **Default to local.** On first run, the suggested profile is a local model if any is discovered.
- **One-click switch.** Profile dropdown in the chat input is always visible.
- **Show capability at a glance.** Each profile card shows badges: `local`, `tool-use`, `vision`, `128k ctx`, etc.
- **Warn on capability mismatch.** Selecting a profile without `tool_use` disables the tool checkboxes with a tooltip.
- **Respect model budgets.** Per-profile budgets (max tokens per turn, max tool calls, max cost) configurable from the same page.

---

## 7. Example — end-to-end "switch from OpenAI to Ollama for this tab"

1. User right-clicks the active tab → "Use profile: local-llama3".
2. The tab's `active_profile_id` updates in `rustacle-settings`.
3. On the next user message, `TurnContext::model_profile` is resolved from the tab override first, falling back to global default.
4. `assemble_prompt(ctx)` uses layer 2 (`model_profile.system_overlay()`) pointing at the `rustacle-llm-local` overlay (see `prompts_catalog.md` §2.3).
5. `host.llm_stream(req)` routes through the Ollama endpoint in the local provider.
6. Reasoning cards render identically; only the cost badge updates to `$0.00` because it's local.

No restart, no recompile, no config file. Two clicks.

---

## 8. Provider + MCP audit table

Every outbound LLM call and every MCP tool invocation is audited:

| Column | Meaning |
|---|---|
| `ts` | timestamp |
| `kind` | `llm_request` / `mcp_call` |
| `provider_id` | `openai` / `anthropic` / `mcp:github-mcp` |
| `endpoint` | URL or command |
| `model` | model id (LLM) / tool name (MCP) |
| `tokens_in` / `tokens_out` | for LLM |
| `cost_usd` | if computable |
| `duration_ms` | |
| `initiator` | `agent:<turn_id>` / `user:palette` / `tool:<name>` |
| `status` | `ok` / `cancelled` / `error:<kind>` |

Exposed in Settings → Audit. Filterable, exportable (typed, secrets excluded).

---
*Related: [README](./README.md) · [agent_reasoning](./agent_reasoning.md) · [architecture](./architecture.md) · [modularity](./modularity.md) · [security](./security.md) · [observability](./observability.md)*
