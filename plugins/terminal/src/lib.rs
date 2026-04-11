pub mod pty;
pub mod tabs;

use bytes::Bytes;
use rustacle_plugin_api::{
    ModuleError, ModuleManifest, PaletteEntry, PanelDesc, RustacleModule, UiContributions,
};
use serde::{Deserialize, Serialize};

use crate::tabs::TabManager;

/// Native terminal plugin. Whitelisted because WASI cannot spawn processes.
pub struct TerminalPlugin {
    manifest: ModuleManifest,
    tabs: TabManager,
}

#[derive(Serialize, Deserialize)]
struct OpenTabRequest {
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Serialize)]
struct OpenTabResponse {
    tab_id: String,
}

#[derive(Deserialize)]
struct WriteRequest {
    tab_id: String,
    data: String,
}

#[derive(Deserialize)]
struct ResizeRequest {
    tab_id: String,
    cols: u16,
    rows: u16,
}

#[derive(Deserialize)]
struct CloseTabRequest {
    tab_id: String,
}

impl TerminalPlugin {
    #[must_use]
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest {
                id: "rustacle.terminal".to_string(),
                name: "Terminal".to_string(),
                version: "0.1.0".to_string(),
                capabilities: vec![rustacle_plugin_api::Capability::Pty],
                subscriptions: vec![],
                ui_contributions: UiContributions {
                    panels: vec![PanelDesc {
                        id: "terminal".to_string(),
                        title: "Terminal".to_string(),
                        icon: Some("terminal".to_string()),
                    }],
                    palette_commands: vec![PaletteEntry {
                        id: "terminal.new_tab".to_string(),
                        label: "Terminal: New Tab".to_string(),
                        shortcut: None,
                    }],
                    settings_schema: None,
                },
            },
            tabs: TabManager::new(),
        }
    }
}

impl Default for TerminalPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RustacleModule for TerminalPlugin {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn init(&mut self) -> Result<(), ModuleError> {
        tracing::info!(plugin.id = self.id(), "terminal plugin initialized");
        Ok(())
    }

    async fn on_event(&mut self, _topic: &str, _payload: Bytes) -> Result<(), ModuleError> {
        Ok(())
    }

    async fn call(&mut self, command: &str, payload: Bytes) -> Result<Bytes, ModuleError> {
        match command {
            "open_tab" => {
                let req: OpenTabRequest =
                    serde_json::from_slice(&payload).map_err(|e| ModuleError::InvalidInput {
                        reason: e.to_string(),
                    })?;
                let tab_id = self.tabs.open_tab(req.cwd.as_deref())?;
                let resp = OpenTabResponse { tab_id };
                serde_json::to_vec(&resp)
                    .map(Bytes::from)
                    .map_err(|e| ModuleError::Internal(e.to_string()))
            }
            "write" => {
                let req: WriteRequest =
                    serde_json::from_slice(&payload).map_err(|e| ModuleError::InvalidInput {
                        reason: e.to_string(),
                    })?;
                self.tabs.write(&req.tab_id, req.data.as_bytes())?;
                Ok(Bytes::from_static(b"{}"))
            }
            "resize" => {
                let req: ResizeRequest =
                    serde_json::from_slice(&payload).map_err(|e| ModuleError::InvalidInput {
                        reason: e.to_string(),
                    })?;
                self.tabs.resize(&req.tab_id, req.cols, req.rows)?;
                Ok(Bytes::from_static(b"{}"))
            }
            "read" => {
                let req: serde_json::Value =
                    serde_json::from_slice(&payload).map_err(|e| ModuleError::InvalidInput {
                        reason: e.to_string(),
                    })?;
                let tab_id = req["tab_id"]
                    .as_str()
                    .ok_or_else(|| ModuleError::InvalidInput {
                        reason: "missing tab_id".to_string(),
                    })?;
                let data = self.tabs.read(tab_id)?;
                Ok(Bytes::from(data))
            }
            "close_tab" => {
                let req: CloseTabRequest =
                    serde_json::from_slice(&payload).map_err(|e| ModuleError::InvalidInput {
                        reason: e.to_string(),
                    })?;
                self.tabs.close_tab(&req.tab_id)?;
                Ok(Bytes::from_static(b"{}"))
            }
            "list_tabs" => {
                let tabs = self.tabs.list_tabs();
                serde_json::to_vec(&tabs)
                    .map(Bytes::from)
                    .map_err(|e| ModuleError::Internal(e.to_string()))
            }
            _ => Err(ModuleError::InvalidInput {
                reason: format!("unknown command: {command}"),
            }),
        }
    }

    async fn shutdown(&mut self) -> Result<(), ModuleError> {
        self.tabs.close_all();
        tracing::info!(plugin.id = self.id(), "terminal plugin shut down");
        Ok(())
    }
}
