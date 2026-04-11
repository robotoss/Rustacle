pub mod layers;
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
/// Layer order is fixed: `system_base` -> `model_profile` -> `env_context` ->
/// tools (via provider dialect) -> `project_docs` -> `selected_files` ->
/// memory -> history + user turn.
#[must_use]
pub fn assemble_prompt(ctx: &TurnContext) -> Prompt {
    let mut p = Prompt::new();

    // 1. Immutable system base
    p.push_system_section("system_base", layers::SYSTEM_BASE);

    // 2. Model profile overlay
    let overlay = layers::render_model_overlay(&ctx.model_profile);
    p.push_system_section("model_profile", &overlay);

    // 2.5. Mode overlay (Plan/Ask — empty for Chat)
    let mode_overlay = layers::render_mode_overlay(ctx);
    p.push_system_section("mode_overlay", &mode_overlay);

    // 3. Environment context
    let env = layers::render_env_context(ctx);
    p.push_system_section("env_context", &env);

    // 4. Tool schemas — filtered by UI-enabled AND permission-granted
    let schemas = tools::collect_tool_schemas(ctx);
    p.set_tools(schemas);

    // 5. Project docs (outermost to innermost)
    for doc in &ctx.project_docs.docs {
        let section_name = format!("project_doc:{}", doc.rel_path);
        let body = layers::truncate_to_budget(&doc.body, layers::PROJECT_DOC_BUDGET);
        p.push_system_section(&section_name, &body);
    }

    // 6. Selected (pinned) files
    for file in &ctx.selected_files {
        let content = layers::render_selected_file(file);
        let section_name = format!("selected_file:{}", file.path.display());
        p.push_system_section(&section_name, &content);
    }

    // 7. Long-term memory
    let memory = layers::render_memory(ctx);
    if !memory.is_empty() {
        p.push_system_section("memory", &memory);
    }

    // 8. System reminders (just before user turn)
    p.push_system_section("system_reminders", layers::SYSTEM_REMINDERS);

    // 9. Conversation history
    let history = layers::render_history(ctx);
    if !history.is_empty() {
        p.push_system_section("history", &history);
    }

    // 10. User turn
    p.push_system_section("user", &ctx.user_turn.text);

    p
}
