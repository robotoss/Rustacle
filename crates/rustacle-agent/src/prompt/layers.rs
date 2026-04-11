//! Prompt layer rendering functions.
//!
//! Each function renders one layer from `TurnContext`. Layers are independent:
//! no layer inspects or mutates another.

use std::fmt::Write;

use rustacle_llm::types::ModelProfile;

use crate::turn_context::{HistoryRole, SelectedFile, TurnContext};

/// Per-file budget for project docs (characters).
pub const PROJECT_DOC_BUDGET: usize = 8_000;

/// Per-file budget for selected files (bytes).
const SELECTED_FILE_BUDGET: usize = 8 * 1024;

/// Default top-K for memory entries.
const MEMORY_TOP_K: usize = 6;

/// Immutable system base. Identity, safety posture, output contract.
/// Must match `prompts_catalog.md` section 1 verbatim.
pub const SYSTEM_BASE: &str = include_str!("text/system_base.txt");

/// System reminders appended just before the user turn.
pub const SYSTEM_REMINDERS: &str = include_str!("text/system_reminders.txt");

/// Render the model profile overlay based on provider type.
#[must_use]
pub fn render_model_overlay(profile: &ModelProfile) -> String {
    let base = match profile.provider.as_str() {
        "openai" => OVERLAY_OPENAI,
        "anthropic" => OVERLAY_ANTHROPIC,
        _ => OVERLAY_LOCAL,
    };
    base.to_owned()
}

const OVERLAY_OPENAI: &str = include_str!("text/overlay_openai.txt");
const OVERLAY_ANTHROPIC: &str = include_str!("text/overlay_anthropic.txt");
const OVERLAY_LOCAL: &str = include_str!("text/overlay_local.txt");

/// Mode overlay texts.
pub const OVERLAY_PLAN_MODE: &str = include_str!("text/overlay_plan_mode.txt");
pub const OVERLAY_ASK_MODE: &str = include_str!("text/overlay_ask_mode.txt");

/// Render mode overlay. Returns empty string for Chat mode (no overlay needed).
#[must_use]
pub fn render_mode_overlay(ctx: &TurnContext) -> String {
    match ctx.extra.get("mode").map(String::as_str) {
        Some("Plan") => OVERLAY_PLAN_MODE.to_owned(),
        Some("Ask") => OVERLAY_ASK_MODE.to_owned(),
        _ => String::new(), // Chat mode: no overlay
    }
}

/// Render the environment context layer from tab/OS snapshots.
#[must_use]
pub fn render_env_context(ctx: &TurnContext) -> String {
    let mut out = String::from("# Environment\n");

    let _ = writeln!(out, "- OS: {} ({})", ctx.host_os.name, ctx.host_os.version);
    let _ = writeln!(
        out,
        "- Shell: {} ({})",
        ctx.active_tab.shell_path, ctx.active_tab.shell_name
    );
    let _ = writeln!(
        out,
        "- Current working directory: {}",
        ctx.active_tab.cwd.display()
    );

    let date = format_date(ctx.now);
    let _ = writeln!(
        out,
        "- Current date: {} (local time zone: {})",
        date, ctx.timezone
    );

    out.push_str("\n# Open terminal tabs\n");
    for tab in &ctx.open_tabs {
        let last = tab
            .last_cmd
            .as_ref()
            .map(|c| format!(", last command: `{}` (exit: {})", c.command, c.exit_code))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "- Tab {} [{}] — cwd: {}, shell: {}{}",
            tab.index,
            tab.title,
            tab.cwd.display(),
            tab.shell_name,
            last
        );
    }

    let _ = write!(
        out,
        "\n# Active tab\nYou are currently targeting **Tab {}** ({}). Tool \
         calls without an explicit `tab_target` will run in this tab. Redirect by \
         setting `tab_target` in the tool arguments.",
        ctx.active_tab.index, ctx.active_tab.title
    );

    out
}

/// Render a selected (pinned) file.
#[must_use]
pub fn render_selected_file(file: &SelectedFile) -> String {
    let body = truncate_to_budget(&file.content, SELECTED_FILE_BUDGET);
    format!(
        "# Pinned file: {}\n```{}\n{}\n```",
        file.path.display(),
        file.language,
        body
    )
}

/// Render memory entries.
#[must_use]
pub fn render_memory(ctx: &TurnContext) -> String {
    let entries = ctx.memory.top_k(MEMORY_TOP_K);
    if entries.is_empty() {
        return String::new();
    }

    let mut out = format!("# Long-term memory (top {})\n", entries.len());
    for entry in entries {
        let _ = writeln!(out, "- ({:.2}) {}", entry.score, entry.text);
    }
    out
}

/// Render conversation history.
#[must_use]
pub fn render_history(ctx: &TurnContext) -> String {
    if ctx.history.messages.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for msg in &ctx.history.messages {
        let role = match msg.role {
            HistoryRole::User => "user",
            HistoryRole::Assistant => "assistant",
            HistoryRole::Tool => "tool",
        };
        let _ = writeln!(out, "[{}]: {}", role, msg.content);
    }
    out
}

/// Truncate text to a character budget, appending a marker if truncated.
#[must_use]
pub fn truncate_to_budget(text: &str, budget: usize) -> String {
    if text.len() <= budget {
        return text.to_owned();
    }
    let mut truncated = text[..budget].to_owned();
    truncated.push_str("\n... (truncated)");
    truncated
}

/// Format a `UnixMillis` timestamp as `YYYY-MM-DD`.
///
/// Simple implementation — no external date library dependency.
fn format_date(millis: u64) -> String {
    let days = millis / 86_400_000;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_date_epoch() {
        assert_eq!(format_date(0), "1970-01-01");
    }

    #[test]
    fn format_date_known() {
        // 2024-01-15 00:00:00 UTC = 1705276800000 ms
        assert_eq!(format_date(1_705_276_800_000), "2024-01-15");
    }

    #[test]
    fn truncate_within_budget() {
        let text = "short";
        assert_eq!(truncate_to_budget(text, 100), "short");
    }

    #[test]
    fn truncate_over_budget() {
        let text = "hello world this is long";
        let result = truncate_to_budget(text, 5);
        assert_eq!(result, "hello\n... (truncated)");
    }
}
