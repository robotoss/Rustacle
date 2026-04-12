//! Prompt registry: tagged, indexed, deterministic prompt assembly.
//!
//! Each prompt is a markdown file with YAML frontmatter. The registry loads
//! built-in prompts at compile time and user prompts from disk at runtime.
//! Assembly filters by role/mode/audience, validates dependencies, and sorts
//! by priority.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Prompt type determines when/how a prompt is included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptType {
    /// Always included if audience matches.
    Section,
    /// Included when the active role matches.
    Role,
    /// Included when the active mode matches.
    Mode,
    /// Per-tool addendum, included when the tool is enabled.
    Tool,
    /// Opt-in by user command or UI toggle.
    Skill,
    /// Full subagent profile for spawning.
    Agent,
}

/// Source of a prompt entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptSource {
    /// Compiled into the binary.
    Builtin,
    /// Loaded from a user directory.
    User(PathBuf),
}

/// YAML frontmatter metadata for a prompt file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub prompt_type: PromptType,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub excludes: Vec<String>,
    #[serde(default = "default_audience")]
    pub audience: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_audience() -> Vec<String> {
    vec!["all".to_owned()]
}

fn default_priority() -> i32 {
    1000
}

/// A single prompt entry: metadata + body text.
#[derive(Debug, Clone)]
pub struct PromptEntry {
    pub meta: PromptMeta,
    pub body: String,
    pub source: PromptSource,
}

/// Errors from registry operations.
#[derive(thiserror::Error, Debug)]
pub enum RegistryError {
    #[error("missing dependency: {id} requires {dep}")]
    MissingDependency { id: String, dep: String },

    #[error("conflict: {id} excludes {other} but both are active")]
    Conflict { id: String, other: String },

    #[error("parse error in {path}: {message}")]
    Parse { path: String, message: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Extract just the body from a markdown string with YAML frontmatter.
/// Used by `layers.rs` constants to strip frontmatter from `include_str!` results.
///
/// # Panics
/// Panics if the content doesn't have valid `---` delimited frontmatter.
#[must_use]
pub fn extract_body(content: &str) -> &str {
    let content = content.trim();
    assert!(
        content.starts_with("---"),
        "prompt file must start with ---"
    );
    let after_first = &content[3..];
    let end_idx = after_first
        .find("\n---")
        .expect("prompt file missing closing ---");
    after_first[end_idx + 4..].trim()
}

/// Central prompt registry. Uses `BTreeMap` for deterministic iteration.
#[derive(Debug, Clone)]
pub struct PromptRegistry {
    entries: BTreeMap<String, PromptEntry>,
}

impl PromptRegistry {
    /// Create a registry with built-in prompts.
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut reg = Self {
            entries: BTreeMap::new(),
        };
        reg.register_builtins();
        reg
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get an entry by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&PromptEntry> {
        self.entries.get(id)
    }

    /// List all entry ids.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }

    /// Insert or override an entry.
    pub fn insert(&mut self, entry: PromptEntry) {
        debug!(id = %entry.meta.id, "prompt registered");
        self.entries.insert(entry.meta.id.clone(), entry);
    }

    /// Load user prompts from a directory. Files must be `.md` with frontmatter.
    /// User entries with the same id override built-in entries.
    ///
    /// # Errors
    /// Returns errors for I/O or parse failures (non-fatal: logs and skips).
    pub fn load_user_dir(&mut self, dir: &Path) -> Vec<RegistryError> {
        let mut errors = Vec::new();

        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    errors.push(RegistryError::Io(e));
                }
                return errors;
            }
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                match Self::parse_file(&path) {
                    Ok(mut prompt_entry) => {
                        prompt_entry.source = PromptSource::User(path.clone());
                        debug!(id = %prompt_entry.meta.id, path = %path.display(), "user prompt loaded");
                        self.entries
                            .insert(prompt_entry.meta.id.clone(), prompt_entry);
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "skipping invalid prompt file");
                        errors.push(e);
                    }
                }
            }
        }

        errors
    }

    /// Resolve prompts for a given role, mode, and active skills.
    /// Returns entries sorted by priority.
    #[must_use]
    pub fn resolve<'a>(
        &'a self,
        role: &str,
        mode: &str,
        active_skills: &[&str],
    ) -> Vec<&'a PromptEntry> {
        let mut result: Vec<&PromptEntry> = self
            .entries
            .values()
            .filter(|e| Self::should_include(e, role, mode, active_skills))
            .collect();

        result.sort_by_key(|e| e.meta.priority);
        result
    }

    /// Validate all dependencies and conflicts in the registry.
    ///
    /// # Errors
    /// Returns a list of validation errors.
    pub fn validate(&self) -> Result<(), Vec<RegistryError>> {
        let mut errors = Vec::new();
        let ids: std::collections::BTreeSet<&str> =
            self.entries.keys().map(String::as_str).collect();

        for entry in self.entries.values() {
            for dep in &entry.meta.requires {
                if !ids.contains(dep.as_str()) {
                    errors.push(RegistryError::MissingDependency {
                        id: entry.meta.id.clone(),
                        dep: dep.clone(),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Check if an entry should be included given the current context.
    fn should_include(entry: &PromptEntry, role: &str, mode: &str, active_skills: &[&str]) -> bool {
        match entry.meta.prompt_type {
            PromptType::Section | PromptType::Tool => {
                Self::audience_matches(&entry.meta.audience, role)
            }
            PromptType::Role => {
                entry.meta.tags.iter().any(|t| t == role)
                    || entry.meta.audience.iter().any(|a| a == role)
            }
            PromptType::Mode => entry.meta.tags.iter().any(|t| t.eq_ignore_ascii_case(mode)),
            PromptType::Skill => active_skills.contains(&entry.meta.id.as_str()),
            PromptType::Agent => false, // agents are resolved explicitly, not auto-included
        }
    }

    /// Check if audience list includes "all" or the specific role.
    fn audience_matches(audience: &[String], role: &str) -> bool {
        audience.iter().any(|a| a == "all" || a == role)
    }

    /// Parse a markdown file with YAML frontmatter.
    fn parse_file(path: &Path) -> Result<PromptEntry, RegistryError> {
        let content = std::fs::read_to_string(path).map_err(RegistryError::Io)?;
        let path_str = path.display().to_string();
        Self::parse_frontmatter(&content, &path_str)
    }

    /// Parse frontmatter from a string. The format is:
    /// ```text
    /// ---
    /// key: value
    /// ---
    /// body text
    /// ```
    fn parse_frontmatter(content: &str, source_name: &str) -> Result<PromptEntry, RegistryError> {
        let content = content.trim();

        if !content.starts_with("---") {
            return Err(RegistryError::Parse {
                path: source_name.to_owned(),
                message: "file must start with ---".to_owned(),
            });
        }

        let after_first = &content[3..];
        let end_idx = after_first
            .find("\n---")
            .ok_or_else(|| RegistryError::Parse {
                path: source_name.to_owned(),
                message: "missing closing --- for frontmatter".to_owned(),
            })?;

        let yaml_str = &after_first[..end_idx];
        let body = after_first[end_idx + 4..].trim().to_owned();

        let meta: PromptMeta =
            serde_yaml::from_str(yaml_str).map_err(|e| RegistryError::Parse {
                path: source_name.to_owned(),
                message: e.to_string(),
            })?;

        Ok(PromptEntry {
            meta,
            body,
            source: PromptSource::Builtin,
        })
    }

    /// Register all built-in prompts by parsing `.md` files with YAML frontmatter.
    /// Each file is included at compile time via `include_str!` and parsed at init.
    /// The `.md` frontmatter is the single source of truth for metadata.
    fn register_builtins(&mut self) {
        /// All built-in `.md` prompt files, included at compile time.
        const BUILTIN_MD_FILES: &[(&str, &str)] = &[
            // Core sections
            (
                "section_identity.md",
                include_str!("text/section_identity.md"),
            ),
            ("section_system.md", include_str!("text/section_system.md")),
            (
                "section_doing_tasks.md",
                include_str!("text/section_doing_tasks.md"),
            ),
            ("section_safety.md", include_str!("text/section_safety.md")),
            (
                "section_cyber_boundary.md",
                include_str!("text/section_cyber_boundary.md"),
            ),
            (
                "section_risk_taxonomy.md",
                include_str!("text/section_risk_taxonomy.md"),
            ),
            (
                "section_actions.md",
                include_str!("text/section_actions.md"),
            ),
            (
                "section_tool_preference.md",
                include_str!("text/section_tool_preference.md"),
            ),
            ("section_tools.md", include_str!("text/section_tools.md")),
            ("section_files.md", include_str!("text/section_files.md")),
            (
                "section_bash_safety.md",
                include_str!("text/section_bash_safety.md"),
            ),
            ("section_shell.md", include_str!("text/section_shell.md")),
            ("section_tone.md", include_str!("text/section_tone.md")),
            ("section_output.md", include_str!("text/section_output.md")),
            (
                "section_loop_avoidance.md",
                include_str!("text/section_loop_avoidance.md"),
            ),
            (
                "section_result_persistence.md",
                include_str!("text/section_result_persistence.md"),
            ),
            // Roles
            ("role_developer.md", include_str!("text/role_developer.md")),
            ("role_manager.md", include_str!("text/role_manager.md")),
            ("role_blogger.md", include_str!("text/role_blogger.md")),
            ("role_analyst.md", include_str!("text/role_analyst.md")),
            ("role_devops.md", include_str!("text/role_devops.md")),
            ("role_designer.md", include_str!("text/role_designer.md")),
            ("role_student.md", include_str!("text/role_student.md")),
            // Modes
            (
                "overlay_plan_mode.md",
                include_str!("text/overlay_plan_mode.md"),
            ),
            (
                "overlay_ask_mode.md",
                include_str!("text/overlay_ask_mode.md"),
            ),
            // System reminders
            (
                "system_reminders.md",
                include_str!("text/system_reminders.md"),
            ),
        ];

        for (filename, content) in BUILTIN_MD_FILES {
            match Self::parse_frontmatter(content, filename) {
                Ok(entry) => {
                    self.entries.insert(entry.meta.id.clone(), entry);
                }
                Err(e) => {
                    // Built-in files must always parse — panic in debug, warn in release.
                    debug_assert!(false, "built-in prompt {filename} failed to parse: {e}");
                    warn!(file = %filename, error = %e, "built-in prompt parse failure");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_load() {
        let reg = PromptRegistry::with_builtins();
        assert!(reg.len() > 20, "expected 20+ builtins, got {}", reg.len());
        assert!(reg.get("section-identity").is_some());
        assert!(reg.get("role-developer").is_some());
        assert!(reg.get("mode-plan").is_some());
    }

    #[test]
    fn resolve_default_developer_chat() {
        let reg = PromptRegistry::with_builtins();
        let resolved = reg.resolve("developer", "Chat", &[]);
        let ids: Vec<&str> = resolved.iter().map(|e| e.meta.id.as_str()).collect();

        assert!(ids.contains(&"section-identity"));
        assert!(ids.contains(&"role-developer"));
        assert!(!ids.contains(&"role-manager"));
        assert!(!ids.contains(&"mode-plan"));
    }

    #[test]
    fn resolve_manager_plan() {
        let reg = PromptRegistry::with_builtins();
        let resolved = reg.resolve("manager", "Plan", &[]);
        let ids: Vec<&str> = resolved.iter().map(|e| e.meta.id.as_str()).collect();

        assert!(ids.contains(&"role-manager"));
        assert!(!ids.contains(&"role-developer"));
        assert!(ids.contains(&"mode-plan"));
    }

    #[test]
    fn resolve_sorted_by_priority() {
        let reg = PromptRegistry::with_builtins();
        let resolved = reg.resolve("developer", "Chat", &[]);
        let priorities: Vec<i32> = resolved.iter().map(|e| e.meta.priority).collect();

        for w in priorities.windows(2) {
            assert!(w[0] <= w[1], "not sorted: {} > {}", w[0], w[1]);
        }
    }

    #[test]
    fn validate_builtins_pass() {
        let reg = PromptRegistry::with_builtins();
        assert!(reg.validate().is_ok());
    }

    #[test]
    fn parse_frontmatter_valid() {
        let content = r#"---
id: "test-section"
name: "Test"
type: section
tags: [test]
priority: 500
---

This is the body text."#;

        let entry = PromptRegistry::parse_frontmatter(content, "test.md").unwrap();
        assert_eq!(entry.meta.id, "test-section");
        assert_eq!(entry.meta.priority, 500);
        assert_eq!(entry.body, "This is the body text.");
    }

    #[test]
    fn parse_frontmatter_missing_closing() {
        let content = "---\nid: test\n\nno closing";
        assert!(PromptRegistry::parse_frontmatter(content, "bad.md").is_err());
    }

    #[test]
    fn user_override_replaces_builtin() {
        let mut reg = PromptRegistry::with_builtins();
        let original = reg.get("section-identity").unwrap().body.clone();

        reg.insert(PromptEntry {
            meta: PromptMeta {
                id: "section-identity".to_owned(),
                name: "Custom Identity".to_owned(),
                description: String::new(),
                prompt_type: PromptType::Section,
                tags: vec!["core".to_owned()],
                requires: Vec::new(),
                excludes: Vec::new(),
                audience: vec!["all".to_owned()],
                priority: 100,
            },
            body: "Custom identity text.".to_owned(),
            source: PromptSource::User(PathBuf::from("test")),
        });

        let updated = reg.get("section-identity").unwrap();
        assert_ne!(updated.body, original);
        assert_eq!(updated.body, "Custom identity text.");
    }
}
