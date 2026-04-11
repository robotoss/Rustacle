use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use rustacle_plugin_api::{
    ModuleError, ModuleManifest, PaletteEntry, PanelDesc, RustacleModule, UiContributions,
};
use serde::{Deserialize, Serialize};

/// A demo native plugin proving the full pipeline: UI → IPC → Kernel → Plugin → back.
pub struct DemoPlugin {
    manifest: ModuleManifest,
    call_count: u64,
}

#[derive(Serialize)]
struct PingResponse {
    message: String,
    plugin_id: String,
    call_count: u64,
    timestamp: u64,
}

#[derive(Deserialize)]
struct EchoRequest {
    text: String,
}

#[derive(Serialize)]
struct EchoResponse {
    echoed: String,
    length: usize,
}

impl DemoPlugin {
    #[must_use]
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest {
                id: "rustacle.demo".to_string(),
                name: "Demo Plugin".to_string(),
                version: "0.1.0".to_string(),
                capabilities: vec![],
                subscriptions: vec![],
                ui_contributions: UiContributions {
                    panels: vec![PanelDesc {
                        id: "demo-panel".to_string(),
                        title: "Demo".to_string(),
                        icon: None,
                    }],
                    palette_commands: vec![PaletteEntry {
                        id: "demo.ping".to_string(),
                        label: "Demo: Ping Plugin".to_string(),
                        shortcut: None,
                    }],
                    settings_schema: None,
                },
            },
            call_count: 0,
        }
    }
}

impl Default for DemoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RustacleModule for DemoPlugin {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn init(&mut self) -> Result<(), ModuleError> {
        tracing::info!(plugin.id = self.id(), "demo plugin initialized");
        Ok(())
    }

    async fn on_event(&mut self, topic: &str, _payload: Bytes) -> Result<(), ModuleError> {
        tracing::debug!(plugin.id = self.id(), topic, "demo plugin received event");
        Ok(())
    }

    async fn call(&mut self, command: &str, payload: Bytes) -> Result<Bytes, ModuleError> {
        self.call_count += 1;

        match command {
            "ping" => {
                #[allow(clippy::cast_possible_truncation)]
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis() as u64);

                let resp = PingResponse {
                    message: "pong from plugin".to_string(),
                    plugin_id: self.id().to_string(),
                    call_count: self.call_count,
                    timestamp,
                };
                serde_json::to_vec(&resp)
                    .map(Bytes::from)
                    .map_err(|e| ModuleError::Internal(e.to_string()))
            }
            "echo" => {
                let req: EchoRequest =
                    serde_json::from_slice(&payload).map_err(|e| ModuleError::InvalidInput {
                        reason: e.to_string(),
                    })?;
                let resp = EchoResponse {
                    length: req.text.len(),
                    echoed: format!("[demo] {}", req.text),
                };
                serde_json::to_vec(&resp)
                    .map(Bytes::from)
                    .map_err(|e| ModuleError::Internal(e.to_string()))
            }
            _ => Err(ModuleError::InvalidInput {
                reason: format!("unknown command: {command}"),
            }),
        }
    }

    async fn shutdown(&mut self) -> Result<(), ModuleError> {
        tracing::info!(
            plugin.id = self.id(),
            calls = self.call_count,
            "demo plugin shutting down"
        );
        Ok(())
    }
}
