//! Per-tab agent context: tracks recent commands and exit codes so the agent
//! prompt can include tab-specific history.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Maximum number of recent commands kept per tab.
const MAX_HISTORY: usize = 50;

/// A single command record in a tab's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    pub command: String,
    pub exit_code: i32,
    pub timestamp_ms: u64,
}

/// Agent-relevant context for one terminal tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabAgentContext {
    pub cwd: String,
    pub command_history: VecDeque<CommandRecord>,
}

impl TabAgentContext {
    #[must_use]
    pub fn new(cwd: &str) -> Self {
        Self {
            cwd: cwd.to_string(),
            command_history: VecDeque::new(),
        }
    }

    /// Record a completed command.
    pub fn push_command(&mut self, command: String, exit_code: i32, timestamp_ms: u64) {
        if self.command_history.len() >= MAX_HISTORY {
            self.command_history.pop_front();
        }
        self.command_history.push_back(CommandRecord {
            command,
            exit_code,
            timestamp_ms,
        });
    }

    /// Update the working directory.
    pub fn set_cwd(&mut self, cwd: String) {
        self.cwd = cwd;
    }

    /// Return the last N commands.
    #[must_use]
    pub fn last_commands(&self, n: usize) -> Vec<&CommandRecord> {
        self.command_history.iter().rev().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_retrieve() {
        let mut ctx = TabAgentContext::new("/home");
        ctx.push_command("ls".into(), 0, 1000);
        ctx.push_command("cargo build".into(), 1, 2000);

        let last = ctx.last_commands(1);
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].command, "cargo build");
        assert_eq!(last[0].exit_code, 1);
    }

    #[test]
    fn respects_max_history() {
        let mut ctx = TabAgentContext::new("/tmp");
        for i in 0..60 {
            ctx.push_command(format!("cmd-{i}"), 0, i);
        }
        assert_eq!(ctx.command_history.len(), MAX_HISTORY);
        assert_eq!(ctx.command_history.front().unwrap().command, "cmd-10");
    }
}
