use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use rustacle_plugin_api::{ModuleError, RustacleModule};
use tokio::sync::RwLock;

type PluginHandle = Arc<RwLock<Box<dyn RustacleModule>>>;

/// Plugin registry. Holds loaded plugin instances and routes commands to them.
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, PluginHandle>>,
}

impl PluginRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a plugin instance.
    pub async fn register(&self, plugin: Box<dyn RustacleModule>) {
        let id = plugin.id().to_string();
        tracing::info!(plugin.id = %id, "plugin registered");
        self.plugins
            .write()
            .await
            .insert(id, Arc::new(RwLock::new(plugin)));
    }

    /// List all registered plugin IDs.
    pub async fn list_ids(&self) -> Vec<String> {
        self.plugins.read().await.keys().cloned().collect()
    }

    /// Route a command to a plugin by ID.
    ///
    /// # Errors
    /// Returns `ModuleError::Internal` if the plugin is not found.
    pub async fn call(
        &self,
        plugin_id: &str,
        command: &str,
        payload: Bytes,
    ) -> Result<Bytes, ModuleError> {
        let plugins = self.plugins.read().await;
        let plugin_arc = plugins
            .get(plugin_id)
            .ok_or_else(|| ModuleError::Internal(format!("plugin not found: {plugin_id}")))?
            .clone();
        drop(plugins); // Release read lock before calling plugin

        let mut plugin = plugin_arc.write().await;
        plugin.call(command, payload).await
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo_plugin::DemoPlugin;

    #[tokio::test]
    async fn registry_register_and_call() {
        let registry = PluginRegistry::new();
        let plugin = DemoPlugin::new();
        registry.register(Box::new(plugin)).await;

        let ids = registry.list_ids().await;
        assert!(ids.contains(&"rustacle.demo".to_string()));

        let result = registry
            .call("rustacle.demo", "ping", Bytes::from_static(b"{}"))
            .await;
        assert!(result.is_ok());

        let response: serde_json::Value =
            serde_json::from_slice(&result.unwrap()).expect("valid json");
        assert_eq!(response["message"], "pong from plugin");
    }

    #[tokio::test]
    async fn registry_call_unknown_plugin() {
        let registry = PluginRegistry::new();
        let result = registry.call("nonexistent", "ping", Bytes::new()).await;
        assert!(result.is_err());
    }
}
