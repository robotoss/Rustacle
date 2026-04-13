use std::collections::HashMap;

use rustacle_plugin_api::ModuleError;
use serde::Serialize;

use crate::pty::PtySession;
use crate::tab_context::TabAgentContext;

/// Manages multiple terminal tabs, each with its own PTY session.
pub struct TabManager {
    tabs: HashMap<String, TabState>,
    tab_order: Vec<String>,
    active_tab_id: Option<String>,
    next_id: u64,
}

/// State for a single terminal tab.
struct TabState {
    session: PtySession,
    cwd: String,
    title: String,
    agent_context: TabAgentContext,
}

/// Serializable tab info for `list_tabs`.
#[derive(Serialize)]
pub struct TabInfo {
    pub id: String,
    pub cwd: String,
    pub title: String,
    pub alive: bool,
    pub index: usize,
    pub active: bool,
}

impl TabManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            tab_order: Vec::new(),
            active_tab_id: None,
            next_id: 0,
        }
    }

    /// Open a new tab with a PTY session.
    ///
    /// # Errors
    /// Returns `ModuleError` if PTY spawn fails.
    pub fn open_tab(&mut self, cwd: Option<&str>) -> Result<String, ModuleError> {
        self.next_id += 1;
        let tab_id = format!("tab-{}", self.next_id);

        let session = PtySession::spawn(cwd, 80, 24)?;

        let cwd_str = cwd.unwrap_or(".").to_string();
        tracing::info!(tab.id = %tab_id, cwd = %cwd_str, "terminal tab opened");

        self.tabs.insert(
            tab_id.clone(),
            TabState {
                session,
                cwd: cwd_str.clone(),
                title: format!("Tab {}", self.next_id),
                agent_context: TabAgentContext::new(&cwd_str),
            },
        );

        self.tab_order.push(tab_id.clone());

        // Auto-activate if this is the first tab.
        if self.active_tab_id.is_none() {
            self.active_tab_id = Some(tab_id.clone());
        }

        Ok(tab_id)
    }

    /// Write data to a tab's PTY.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist or write fails.
    pub fn write(&mut self, tab_id: &str, data: &[u8]) -> Result<(), ModuleError> {
        let tab = self.get_tab_mut(tab_id)?;
        tab.session.write(data)
    }

    /// Read buffered output from a tab's PTY (non-blocking).
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist.
    pub fn read(&mut self, tab_id: &str) -> Result<Vec<u8>, ModuleError> {
        let tab = self.get_tab_mut(tab_id)?;
        Ok(tab.session.read())
    }

    /// Resize a tab's PTY.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist or resize fails.
    pub fn resize(&mut self, tab_id: &str, cols: u16, rows: u16) -> Result<(), ModuleError> {
        let tab = self.get_tab_mut(tab_id)?;
        tab.session.resize(cols, rows)
    }

    /// Close a tab and kill its PTY.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist.
    pub fn close_tab(&mut self, tab_id: &str) -> Result<(), ModuleError> {
        self.tabs
            .remove(tab_id)
            .ok_or_else(|| ModuleError::InvalidInput {
                reason: format!("tab not found: {tab_id}"),
            })?;

        self.tab_order.retain(|id| id != tab_id);

        // If we closed the active tab, activate the previous or next one.
        if self.active_tab_id.as_deref() == Some(tab_id) {
            self.active_tab_id = self.tab_order.last().cloned();
        }

        tracing::info!(tab.id = %tab_id, "terminal tab closed");
        Ok(())
    }

    /// List all open tabs in order.
    pub fn list_tabs(&mut self) -> Vec<TabInfo> {
        self.tab_order
            .iter()
            .enumerate()
            .filter_map(|(idx, id)| {
                let state = self.tabs.get_mut(id)?;
                Some(TabInfo {
                    id: id.clone(),
                    cwd: state.cwd.clone(),
                    title: state.title.clone(),
                    alive: state.session.is_alive(),
                    index: idx,
                    active: self.active_tab_id.as_deref() == Some(id.as_str()),
                })
            })
            .collect()
    }

    /// Close all tabs.
    pub fn close_all(&mut self) {
        self.tabs.clear();
        self.tab_order.clear();
        self.active_tab_id = None;
        tracing::info!("all terminal tabs closed");
    }

    /// Reorder a tab to a new index position.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist or index is out of
    /// bounds.
    pub fn reorder_tab(&mut self, tab_id: &str, new_index: usize) -> Result<(), ModuleError> {
        let old_index = self
            .tab_order
            .iter()
            .position(|id| id == tab_id)
            .ok_or_else(|| ModuleError::InvalidInput {
                reason: format!("tab not found: {tab_id}"),
            })?;

        if new_index >= self.tab_order.len() {
            return Err(ModuleError::InvalidInput {
                reason: format!(
                    "index {new_index} out of range (0..{})",
                    self.tab_order.len()
                ),
            });
        }

        let id = self.tab_order.remove(old_index);
        self.tab_order.insert(new_index, id);

        tracing::debug!(tab.id = %tab_id, old_index, new_index, "tab reordered");
        Ok(())
    }

    /// Set the active tab.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist.
    pub fn set_active_tab(&mut self, tab_id: &str) -> Result<(), ModuleError> {
        if !self.tabs.contains_key(tab_id) {
            return Err(ModuleError::InvalidInput {
                reason: format!("tab not found: {tab_id}"),
            });
        }
        self.active_tab_id = Some(tab_id.to_string());
        Ok(())
    }

    /// Set a tab's title.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist.
    pub fn set_tab_title(&mut self, tab_id: &str, title: String) -> Result<(), ModuleError> {
        let tab = self.get_tab_mut(tab_id)?;
        tab.title = title;
        Ok(())
    }

    /// Get the active tab ID.
    #[must_use]
    pub fn active_tab(&self) -> Option<&str> {
        self.active_tab_id.as_deref()
    }

    /// Get agent context for a tab.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist.
    pub fn get_agent_context(&self, tab_id: &str) -> Result<&TabAgentContext, ModuleError> {
        self.tabs
            .get(tab_id)
            .map(|t| &t.agent_context)
            .ok_or_else(|| ModuleError::InvalidInput {
                reason: format!("tab not found: {tab_id}"),
            })
    }

    /// Record a command in a tab's agent context.
    ///
    /// # Errors
    /// Returns `ModuleError` if the tab doesn't exist.
    pub fn push_command(
        &mut self,
        tab_id: &str,
        command: String,
        exit_code: i32,
        timestamp_ms: u64,
    ) -> Result<(), ModuleError> {
        let tab = self.get_tab_mut(tab_id)?;
        tab.agent_context
            .push_command(command, exit_code, timestamp_ms);
        Ok(())
    }

    fn get_tab_mut(&mut self, tab_id: &str) -> Result<&mut TabState, ModuleError> {
        self.tabs
            .get_mut(tab_id)
            .ok_or_else(|| ModuleError::InvalidInput {
                reason: format!("tab not found: {tab_id}"),
            })
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: These tests cannot run without a real PTY, so they only test
    // ordering/reorder logic on an empty manager. Full integration tests
    // require the Tauri runtime.

    #[test]
    fn tab_order_maintained() {
        let mut mgr = TabManager::new();
        // Simulate adding to order without PTY.
        mgr.tab_order.push("tab-1".into());
        mgr.tab_order.push("tab-2".into());
        mgr.tab_order.push("tab-3".into());

        assert_eq!(mgr.tab_order, vec!["tab-1", "tab-2", "tab-3"]);
    }

    #[test]
    fn reorder_moves_tab() {
        let mut mgr = TabManager::new();
        // Insert stubs so reorder doesn't fail on missing tab.
        for name in ["tab-1", "tab-2", "tab-3"] {
            mgr.tab_order.push(name.into());
            // We don't actually need PtySession for reorder.
        }

        // Move tab-3 to index 0 — need it in the tabs map for the check.
        // reorder only checks tab_order position, but let's keep it clean.
        assert!(mgr.reorder_tab("tab-3", 0).is_ok());
        assert_eq!(mgr.tab_order, vec!["tab-3", "tab-1", "tab-2"]);
    }
}
