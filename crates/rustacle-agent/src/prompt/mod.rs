pub mod layers;
pub mod registry;
pub mod tools;

#[cfg(test)]
mod golden_tests;

use rustacle_llm::types::ToolSchema;

use crate::turn_context::TurnContext;

/// Assembled prompt ready for the LLM provider.
#[derive(Debug, Clone)]
pub struct Prompt {
    /// System message sections, in assembly order.
    sections: Vec<PromptSection>,
    /// Tool schemas in provider's native dialect.
    tool_schemas: Vec<ToolSchema>,
}

/// A named section within the system message.
#[derive(Debug, Clone)]
struct PromptSection {
    name: String,
    content: String,
}

impl Prompt {
    fn new() -> Self {
        Self {
            sections: Vec::new(),
            tool_schemas: Vec::new(),
        }
    }

    fn push_system_section(&mut self, name: &str, content: &str) {
        if content.is_empty() {
            return;
        }
        self.sections.push(PromptSection {
            name: name.to_owned(),
            content: content.to_owned(),
        });
    }

    fn set_tools(&mut self, schemas: Vec<ToolSchema>) {
        self.tool_schemas = schemas;
    }

    /// Get tool schemas (for the provider's tool-use parameter).
    #[must_use]
    pub fn tool_schemas(&self) -> &[ToolSchema] {
        &self.tool_schemas
    }

    /// Get section names (for diff testing).
    #[must_use]
    pub fn section_names(&self) -> Vec<&str> {
        self.sections.iter().map(|s| s.name.as_str()).collect()
    }

    /// Render as a single system-message string with `## section` headers.
    #[must_use]
    pub fn to_system_message(&self) -> String {
        let mut out = String::new();
        for (i, section) in self.sections.iter().enumerate() {
            if i > 0 {
                out.push_str("\n\n");
            }
            out.push_str("## ");
            out.push_str(&section.name);
            out.push_str("\n\n");
            out.push_str(&section.content);
        }
        out
    }
}

impl std::fmt::Display for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_system_message())
    }
}

/// Build a deterministic, layered prompt from the turn context.
///
/// Given identical `TurnContext`, two calls produce byte-identical output.
/// Layer order is fixed: identity, system, role, tasks, safety, actions,
/// tools, files, shell, tone, output, model profile, mode, env, tool schemas,
/// project docs, selected files, memory, reminders, history, user turn.
#[must_use]
pub fn assemble_prompt(ctx: &TurnContext) -> Prompt {
    let mut p = Prompt::new();

    // === STATIC SECTIONS (cacheable across turns) ===

    // 1. Identity — who the agent is
    p.push_system_section("identity", layers::SECTION_IDENTITY);

    // 2. System — how the UI/tool pipeline works
    p.push_system_section("system", layers::SECTION_SYSTEM);

    // 3. Role overlay — audience-specific (developer/manager)
    let role = layers::render_role_overlay(ctx);
    p.push_system_section("role", &role);

    // 4. Doing tasks — coding discipline, anti-patterns, quality
    p.push_system_section("doing_tasks", layers::SECTION_DOING_TASKS);

    // 5. Safety posture — credentials, destructive actions, security
    p.push_system_section("safety", layers::SECTION_SAFETY);

    // 6. Executing actions with care — reversibility, blast radius
    p.push_system_section("actions", layers::SECTION_ACTIONS);

    // 7. Tool usage discipline — prefer dedicated tools, parallelism
    p.push_system_section("tools", layers::SECTION_TOOLS);

    // 8. Working with files
    p.push_system_section("files", layers::SECTION_FILES);

    // 9. Working with the shell
    p.push_system_section("shell", layers::SECTION_SHELL);

    // 10. Tone and style
    p.push_system_section("tone", layers::SECTION_TONE);

    // 11. Output efficiency — conciseness, structure
    p.push_system_section("output", layers::SECTION_OUTPUT);

    // 12. Model profile overlay (provider-specific dialect)
    let overlay = layers::render_model_overlay(&ctx.model_profile);
    p.push_system_section("model_profile", &overlay);

    // === DYNAMIC SECTIONS (per-turn) ===

    // 13. Mode overlay (Plan/Ask — empty for Chat)
    let mode_overlay = layers::render_mode_overlay(ctx);
    p.push_system_section("mode_overlay", &mode_overlay);

    // 14. Environment context (OS, shell, tabs, cwd)
    let env = layers::render_env_context(ctx);
    p.push_system_section("env_context", &env);

    // 15. Tool schemas — filtered by UI-enabled AND permission-granted
    let schemas = tools::collect_tool_schemas(ctx);
    p.set_tools(schemas);

    // 16. Project docs (outermost to innermost)
    for doc in &ctx.project_docs.docs {
        let section_name = format!("project_doc:{}", doc.rel_path);
        let body = layers::truncate_to_budget(&doc.body, layers::PROJECT_DOC_BUDGET);
        p.push_system_section(&section_name, &body);
    }

    // 17. Selected (pinned) files
    for file in &ctx.selected_files {
        let content = layers::render_selected_file(file);
        let section_name = format!("selected_file:{}", file.path.display());
        p.push_system_section(&section_name, &content);
    }

    // 18. Long-term memory
    let memory = layers::render_memory(ctx);
    if !memory.is_empty() {
        p.push_system_section("memory", &memory);
    }

    // 19. System reminders (loop avoidance, conciseness nudges)
    p.push_system_section("system_reminders", layers::SYSTEM_REMINDERS);

    // 20. Conversation history
    let history = layers::render_history(ctx);
    if !history.is_empty() {
        p.push_system_section("history", &history);
    }

    // 21. User turn
    p.push_system_section("user", &ctx.user_turn.text);

    p
}

/// Registry-based prompt assembly. Resolves tagged prompts by role/mode,
/// then appends dynamic per-turn sections.
///
/// This is the preferred assembly path. `assemble_prompt()` (v1) is kept
/// for backward compatibility and golden tests.
#[must_use]
pub fn assemble_prompt_v2(ctx: &TurnContext, reg: &registry::PromptRegistry) -> Prompt {
    let role = ctx.extra.get("role").map_or("developer", String::as_str);
    let mode = ctx.extra.get("mode").map_or("Chat", String::as_str);

    // Parse active skills from comma-separated string
    let skills_str = ctx.extra.get("active_skills").map_or("", String::as_str);
    let active_skills: Vec<&str> = if skills_str.is_empty() {
        Vec::new()
    } else {
        skills_str.split(',').map(str::trim).collect()
    };

    let mut p = Prompt::new();

    // === REGISTRY-RESOLVED SECTIONS (static + role + mode + skills) ===
    for entry in reg.resolve(role, mode, &active_skills) {
        p.push_system_section(&entry.meta.id, &entry.body);
    }

    // === MODEL PROFILE (depends on ctx.model_profile, not registry) ===
    let overlay = layers::render_model_overlay(&ctx.model_profile);
    p.push_system_section("model_profile", &overlay);

    // === DYNAMIC PER-TURN SECTIONS ===

    // Environment context (OS, shell, tabs, cwd)
    let env = layers::render_env_context(ctx);
    p.push_system_section("env_context", &env);

    // Tool schemas
    let schemas = tools::collect_tool_schemas(ctx);
    p.set_tools(schemas);

    // Project docs
    for doc in &ctx.project_docs.docs {
        let section_name = format!("project_doc:{}", doc.rel_path);
        let body = layers::truncate_to_budget(&doc.body, layers::PROJECT_DOC_BUDGET);
        p.push_system_section(&section_name, &body);
    }

    // Selected (pinned) files
    for file in &ctx.selected_files {
        let content = layers::render_selected_file(file);
        let section_name = format!("selected_file:{}", file.path.display());
        p.push_system_section(&section_name, &content);
    }

    // Long-term memory
    let memory = layers::render_memory(ctx);
    if !memory.is_empty() {
        p.push_system_section("memory", &memory);
    }

    // System reminders
    p.push_system_section("system_reminders", layers::SYSTEM_REMINDERS);

    // Conversation history
    let history = layers::render_history(ctx);
    if !history.is_empty() {
        p.push_system_section("history", &history);
    }

    // User turn
    p.push_system_section("user", &ctx.user_turn.text);

    p
}
