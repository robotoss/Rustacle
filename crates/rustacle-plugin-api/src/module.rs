use bytes::Bytes;

use crate::errors::ModuleError;
use crate::manifest::ModuleManifest;

/// Host-side handle to a loaded plugin (WASM or native).
///
/// Plugin authors do NOT implement this trait — they export the WIT `module`
/// interface, and `rustacle-wasm-host` adapts it into this trait.
#[async_trait::async_trait]
pub trait RustacleModule: Send + Sync {
    /// Unique plugin identifier.
    fn id(&self) -> &str;

    /// The plugin's manifest (identity, capabilities, UI contributions).
    fn manifest(&self) -> &ModuleManifest;

    /// Initialize the plugin after capability negotiation.
    ///
    /// # Errors
    /// Returns `ModuleError` if initialization fails.
    async fn init(&mut self) -> Result<(), ModuleError>;

    /// Deliver an event bus message to the plugin.
    ///
    /// # Errors
    /// Returns `ModuleError` if the plugin cannot process the event.
    async fn on_event(&mut self, topic: &str, payload: Bytes) -> Result<(), ModuleError>;

    /// Invoke a plugin command by name with a typed payload.
    ///
    /// # Errors
    /// Returns `ModuleError` if the command fails.
    async fn call(&mut self, command: &str, payload: Bytes) -> Result<Bytes, ModuleError>;

    /// Gracefully shut down the plugin.
    ///
    /// # Errors
    /// Returns `ModuleError` if shutdown fails.
    async fn shutdown(&mut self) -> Result<(), ModuleError>;

    /// Export the plugin's state for hot-swap migration.
    /// Returns `None` for `Transient` state policy.
    async fn export_state(&self) -> Option<Bytes> {
        None
    }

    /// Import state from a previous plugin version during hot-swap.
    ///
    /// # Errors
    /// Returns `ModuleError` if state import fails (abort swap, keep old instance).
    async fn import_state(&mut self, _state: Bytes) -> Result<(), ModuleError> {
        Ok(())
    }
}
