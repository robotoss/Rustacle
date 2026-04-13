pub mod pty;
pub mod splits;
pub mod tab_context;
pub mod tabs;

use bytes::Bytes;
use rustacle_plugin_api::{
    ModuleError, ModuleManifest, PaletteEntry, PanelDesc, RustacleModule, UiContributions,
};
use serde::{Deserialize, Serialize};

use crate::splits::{SplitDirection, SplitTree};
use crate::tabs::TabManager;

/// Native terminal plugin. Whitelisted because WASI cannot spawn processes.
pub struct TerminalPlugin {
    manifest: ModuleManifest,
    tabs: TabManager,
    splits: SplitTree,
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

#[derive(Deserialize)]
struct SplitTabRequest {
    tab_id: String,
    direction: String,
}

#[derive(Serialize)]
struct SplitTabResponse {
    new_tab_id: String,
    split_node_id: String,
}

#[derive(Deserialize)]
struct ResizeSplitRequest {
    node_id: String,
    ratio: f64,
}

#[derive(Deserialize)]
struct ReorderTabRequest {
    tab_id: String,
    new_index: usize,
}

#[derive(Deserialize)]
struct SetActiveTabRequest {
    tab_id: String,
}

#[derive(Deserialize)]
struct SetTabTitleRequest {
    tab_id: String,
    title: String,
}

#[derive(Deserialize)]
struct GetTabContextRequest {
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
            splits: SplitTree::new(),
        }
    }
}

impl Default for TerminalPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to deserialize a JSON payload or return `InvalidInput`.
fn parse_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, ModuleError> {
    serde_json::from_slice(payload).map_err(|e| ModuleError::InvalidInput {
        reason: e.to_string(),
    })
}

/// Serialize a value to `Bytes` or return `Internal`.
fn json_bytes<T: Serialize>(val: &T) -> Result<Bytes, ModuleError> {
    serde_json::to_vec(val)
        .map(Bytes::from)
        .map_err(|e| ModuleError::Internal(e.to_string()))
}

/// Empty JSON response.
const OK_EMPTY: &[u8] = b"{}";

impl TerminalPlugin {
    fn handle_tab_commands(
        &mut self,
        command: &str,
        payload: &Bytes,
    ) -> Result<Bytes, ModuleError> {
        match command {
            "open_tab" => {
                let req: OpenTabRequest = parse_payload(payload)?;
                let tab_id = self.tabs.open_tab(req.cwd.as_deref())?;
                self.splits.insert_leaf(&tab_id);
                json_bytes(&OpenTabResponse {
                    tab_id: tab_id.clone(),
                })
            }
            "write" => {
                let req: WriteRequest = parse_payload(payload)?;
                self.tabs.write(&req.tab_id, req.data.as_bytes())?;
                Ok(Bytes::from_static(OK_EMPTY))
            }
            "resize" => {
                let req: ResizeRequest = parse_payload(payload)?;
                self.tabs.resize(&req.tab_id, req.cols, req.rows)?;
                Ok(Bytes::from_static(OK_EMPTY))
            }
            "read" => {
                let req: serde_json::Value = parse_payload(payload)?;
                let tab_id = req["tab_id"]
                    .as_str()
                    .ok_or_else(|| ModuleError::InvalidInput {
                        reason: "missing tab_id".to_string(),
                    })?;
                let data = self.tabs.read(tab_id)?;
                Ok(Bytes::from(data))
            }
            "close_tab" => {
                let req: CloseTabRequest = parse_payload(payload)?;
                self.tabs.close_tab(&req.tab_id)?;
                let _ = self.splits.close_leaf(&req.tab_id);
                Ok(Bytes::from_static(OK_EMPTY))
            }
            "list_tabs" => json_bytes(&self.tabs.list_tabs()),
            "reorder_tab" => {
                let req: ReorderTabRequest = parse_payload(payload)?;
                self.tabs.reorder_tab(&req.tab_id, req.new_index)?;
                Ok(Bytes::from_static(OK_EMPTY))
            }
            "set_active_tab" => {
                let req: SetActiveTabRequest = parse_payload(payload)?;
                self.tabs.set_active_tab(&req.tab_id)?;
                Ok(Bytes::from_static(OK_EMPTY))
            }
            "set_tab_title" => {
                let req: SetTabTitleRequest = parse_payload(payload)?;
                self.tabs.set_tab_title(&req.tab_id, req.title)?;
                Ok(Bytes::from_static(OK_EMPTY))
            }
            "get_tab_context" => {
                let req: GetTabContextRequest = parse_payload(payload)?;
                let ctx = self.tabs.get_agent_context(&req.tab_id)?;
                json_bytes(ctx)
            }
            _ => Err(ModuleError::InvalidInput {
                reason: format!("unknown tab command: {command}"),
            }),
        }
    }

    fn handle_split_commands(
        &mut self,
        command: &str,
        payload: &Bytes,
    ) -> Result<Bytes, ModuleError> {
        match command {
            "split_tab" => {
                let req: SplitTabRequest = parse_payload(payload)?;
                let direction = match req.direction.as_str() {
                    "horizontal" => SplitDirection::Horizontal,
                    "vertical" => SplitDirection::Vertical,
                    other => {
                        return Err(ModuleError::InvalidInput {
                            reason: format!(
                                "invalid direction: {other} (expected horizontal|vertical)"
                            ),
                        });
                    }
                };
                let source_cwd = self.tabs.get_agent_context(&req.tab_id)?.cwd.clone();
                let new_tab_id = self.tabs.open_tab(Some(&source_cwd))?;
                let split_node_id = self.splits.split_tab(&req.tab_id, &new_tab_id, direction)?;
                json_bytes(&SplitTabResponse {
                    new_tab_id,
                    split_node_id,
                })
            }
            "resize_split" => {
                let req: ResizeSplitRequest = parse_payload(payload)?;
                self.splits.resize_split(&req.node_id, req.ratio)?;
                Ok(Bytes::from_static(OK_EMPTY))
            }
            "get_layout" => json_bytes(&self.splits.to_layout()),
            _ => Err(ModuleError::InvalidInput {
                reason: format!("unknown split command: {command}"),
            }),
        }
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
            "split_tab" | "resize_split" | "get_layout" => {
                self.handle_split_commands(command, &payload)
            }
            _ => self.handle_tab_commands(command, &payload),
        }
    }

    async fn shutdown(&mut self) -> Result<(), ModuleError> {
        self.tabs.close_all();
        tracing::info!(plugin.id = self.id(), "terminal plugin shut down");
        Ok(())
    }
}
