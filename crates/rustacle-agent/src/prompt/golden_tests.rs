//! Golden (snapshot) tests for prompt assembly determinism.
//!
//! Uses `insta` to capture and verify byte-identical prompt output.
//! Any change to prompt layers requires updating snapshots via `cargo insta review`.

use std::path::PathBuf;

use crate::prompt::assemble_prompt;
use crate::turn_context::{
    CommandRecord, ConversationHistory, HistoryMessage, HistoryRole, HostOs, MemoryEntry,
    MemoryView, PermissionView, ProjectDoc, ProjectDocs, SelectedFile, TabSnapshot, TabSummary,
    TurnContext, UserMessage,
};

/// Baseline fixture: a typical turn with all layers populated.
fn turn_context_fixture_a() -> TurnContext {
    TurnContext {
        turn_id: "turn-001".to_owned(),
        user_turn: UserMessage {
            text: "find all TODOs in the src dir and summarize".to_owned(),
        },
        history: ConversationHistory {
            messages: vec![
                HistoryMessage {
                    role: HistoryRole::User,
                    content: "hello".to_owned(),
                },
                HistoryMessage {
                    role: HistoryRole::Assistant,
                    content: "Hi! How can I help?".to_owned(),
                },
            ],
        },
        model_profile: rustacle_llm::ModelProfile {
            name: "default".to_owned(),
            provider: "anthropic".to_owned(),
            model: "claude-sonnet-4-20250514".to_owned(),
            api_base: None,
            max_tokens: Some(4096),
            temperature: Some(0.0),
        },
        ui_enabled_tools: vec![
            "bash".to_owned(),
            "fs_read".to_owned(),
            "grep".to_owned(),
            "glob".to_owned(),
        ],
        active_tab: TabSnapshot {
            index: 0,
            title: "main".to_owned(),
            cwd: PathBuf::from("/home/k/projects/rustacle"),
            shell_name: "zsh".to_owned(),
            shell_path: "/bin/zsh".to_owned(),
            last_commands: vec![CommandRecord {
                command: "cargo build".to_owned(),
                exit_code: 0,
            }],
        },
        open_tabs: vec![
            TabSummary {
                index: 0,
                title: "main".to_owned(),
                cwd: PathBuf::from("/home/k/projects/rustacle"),
                shell_name: "zsh".to_owned(),
                last_cmd: Some(CommandRecord {
                    command: "cargo build".to_owned(),
                    exit_code: 0,
                }),
            },
            TabSummary {
                index: 1,
                title: "logs".to_owned(),
                cwd: PathBuf::from("/var/log"),
                shell_name: "bash".to_owned(),
                last_cmd: Some(CommandRecord {
                    command: "tail -f syslog".to_owned(),
                    exit_code: 0,
                }),
            },
        ],
        host_os: HostOs {
            name: "Linux".to_owned(),
            version: "6.8.0".to_owned(),
        },
        permissions: PermissionView {
            granted_tools: vec![
                "bash".to_owned(),
                "fs_read".to_owned(),
                "glob".to_owned(),
                "grep".to_owned(),
            ],
        },
        project_docs: ProjectDocs {
            docs: vec![ProjectDoc {
                rel_path: "RUSTACLE.md".to_owned(),
                body: "# Rustacle\n\nA local-first desktop agent controller.".to_owned(),
            }],
        },
        memory: MemoryView {
            entries: vec![
                MemoryEntry {
                    text: "user prefers one bundled PR over many small ones for refactors"
                        .to_owned(),
                    score: 0.87,
                },
                MemoryEntry {
                    text: "project uses conventional commits: feat(scope): ...".to_owned(),
                    score: 0.72,
                },
            ],
        },
        selected_files: vec![SelectedFile {
            path: PathBuf::from("src/main.rs"),
            content: "fn main() {\n    println!(\"hello\");\n}".to_owned(),
            language: "rust".to_owned(),
        }],
        // Pinned to 2024-01-15 00:00:00 UTC for reproducibility
        now: 1_705_276_800_000,
        timezone: "Europe/Moscow".to_owned(),
        extra: Default::default(),
    }
}

/// Same as fixture_a but with a different cwd.
fn turn_context_fixture_a_with_cwd(cwd: &str) -> TurnContext {
    let mut ctx = turn_context_fixture_a();
    ctx.active_tab.cwd = PathBuf::from(cwd);
    ctx
}

#[test]
fn prompt_is_byte_identical_for_fixed_context() {
    let ctx = turn_context_fixture_a();
    let prompt = assemble_prompt(&ctx);
    insta::assert_snapshot!(prompt.to_system_message());
}

#[test]
fn prompt_deterministic_across_calls() {
    let ctx = turn_context_fixture_a();
    let a = assemble_prompt(&ctx).to_system_message();
    let b = assemble_prompt(&ctx).to_system_message();
    assert_eq!(
        a, b,
        "Two calls with identical TurnContext must produce identical output"
    );
}

#[test]
fn changing_cwd_changes_only_env_layer() {
    let a = turn_context_fixture_a();
    let b = turn_context_fixture_a_with_cwd("/tmp");

    let prompt_a = assemble_prompt(&a);
    let prompt_b = assemble_prompt(&b);

    let sections_a = prompt_a.section_names();
    let sections_b = prompt_b.section_names();
    assert_eq!(
        sections_a, sections_b,
        "Section structure must be identical"
    );

    // Find which sections differ
    let sys_a = prompt_a.to_system_message();
    let sys_b = prompt_b.to_system_message();

    let lines_a: Vec<&str> = sys_a.lines().collect();
    let lines_b: Vec<&str> = sys_b.lines().collect();

    let mut changed_sections = Vec::new();
    let mut current_section = "";

    for (la, lb) in lines_a.iter().zip(lines_b.iter()) {
        if let Some(name) = la.strip_prefix("## ") {
            current_section = name;
        }
        if la != lb && !changed_sections.contains(&current_section) {
            changed_sections.push(current_section);
        }
    }

    assert_eq!(
        changed_sections,
        vec!["env_context"],
        "Only env_context should change when cwd changes"
    );
}

#[test]
fn tools_filtered_by_permission() {
    let mut ctx = turn_context_fixture_a();
    // Remove grep from permissions
    ctx.permissions.granted_tools.retain(|t| t != "grep");

    let prompt = assemble_prompt(&ctx);
    let tool_names: Vec<&str> = prompt
        .tool_schemas()
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert!(!tool_names.contains(&"grep"), "grep should be filtered out");
    assert!(tool_names.contains(&"bash"), "bash should remain");
}

#[test]
fn empty_memory_omits_section() {
    let mut ctx = turn_context_fixture_a();
    ctx.memory = MemoryView::default();

    let prompt = assemble_prompt(&ctx);
    let sections = prompt.section_names();
    assert!(
        !sections.contains(&"memory"),
        "Empty memory should not produce a section"
    );
}
