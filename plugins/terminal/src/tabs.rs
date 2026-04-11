use std::collections::HashMap;

use rustacle_plugin_api::ModuleError;
use serde::Serialize;

use crate::pty::PtySession;

/// Manages multiple terminal tabs, each with its own PTY session.
pub struct TabManager {
    tabs: HashMap<String, TabState>,
    next_id: u64,
}

/// State for a single terminal tab.
struct TabState {
    session: PtySession,
    cwd: String,
}

/// Serializable tab info for `list_tabs`.
#[derive(Serialize)]
pub struct TabInfo {
    pub id: String,
    pub cwd: String,
    pub alive: bool,
}

impl TabManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
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
                cwd: cwd_str,
            },
        );

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
        tracing::info!(tab.id = %tab_id, "terminal tab closed");
        Ok(())
    }

    /// List all open tabs.
    pub fn list_tabs(&mut self) -> Vec<TabInfo> {
        self.tabs
            .iter_mut()
            .map(|(id, state)| TabInfo {
                id: id.clone(),
                cwd: state.cwd.clone(),
                alive: state.session.is_alive(),
            })
            .collect()
    }

    /// Close all tabs.
    pub fn close_all(&mut self) {
        self.tabs.clear();
        tracing::info!("all terminal tabs closed");
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
