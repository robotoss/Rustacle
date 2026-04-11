mod ask;
mod key;

pub use ask::{Grant, PermissionAsk, PermissionDecision};
pub use key::CapabilityKey;

use dashmap::DashMap;
use rustacle_plugin_api::{Capability, ModuleError};
use tokio::sync::mpsc;

/// Central permission broker. Every capability use passes through here.
pub struct PermissionBroker {
    grants: DashMap<(String, CapabilityKey), Grant>,
    ask_tx: mpsc::Sender<PermissionAsk>,
}

impl PermissionBroker {
    /// Create a new broker. Returns the broker and a receiver for permission asks.
    #[must_use]
    pub fn new() -> (Self, mpsc::Receiver<PermissionAsk>) {
        let (ask_tx, ask_rx) = mpsc::channel(64);
        let broker = Self {
            grants: DashMap::new(),
            ask_tx,
        };
        (broker, ask_rx)
    }

    /// Check if a plugin has a granted capability.
    /// Returns immediately if cached; otherwise sends a `PermissionAsk` to the UI.
    ///
    /// # Errors
    /// Returns `ModuleError::Denied` if the user denies the capability.
    pub async fn check(
        &self,
        plugin_id: &str,
        cap: &Capability,
    ) -> Result<(), ModuleError> {
        let key = CapabilityKey::from(cap);
        let cache_key = (plugin_id.to_string(), key.clone());

        // Check cache first.
        if let Some(grant) = self.grants.get(&cache_key) {
            tracing::trace!(plugin.id = plugin_id, capability = ?key, "permission cache hit");
            return grant.as_result(cap);
        }

        // Not cached — ask the user.
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let ask = PermissionAsk {
            plugin_id: plugin_id.to_string(),
            capability: cap.clone(),
            reply_tx,
        };

        self.ask_tx
            .send(ask)
            .await
            .map_err(|_| ModuleError::Internal("permission channel closed".to_string()))?;

        let decision = reply_rx
            .await
            .map_err(|_| ModuleError::Internal("permission reply dropped".to_string()))?;

        let grant = Grant { decision };

        // Only cache Allow decisions — denials can be retried.
        if matches!(grant.decision, PermissionDecision::Allow | PermissionDecision::AllowSession) {
            self.grants.insert(cache_key, grant.clone());
        }

        grant.as_result(cap)
    }

    /// Invalidate a cached grant (e.g. when user edits permissions in Settings).
    pub fn invalidate(&self, plugin_id: &str, key: &CapabilityKey) {
        self.grants.remove(&(plugin_id.to_string(), key.clone()));
        tracing::info!(plugin.id = plugin_id, capability = ?key, "permission grant invalidated");
    }

    /// Invalidate all grants for a plugin.
    pub fn invalidate_all(&self, plugin_id: &str) {
        self.grants.retain(|k, _| k.0 != plugin_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustacle_plugin_api::{FsMode, PathScope};

    #[tokio::test]
    async fn permission_allow_is_cached() {
        let (broker, mut ask_rx) = PermissionBroker::new();
        let cap = Capability::Fs {
            scope: PathScope::new(std::path::Path::new("/tmp/test")),
            mode: FsMode::ReadOnly,
        };

        // Spawn a task to answer the permission ask.
        tokio::spawn(async move {
            if let Some(ask) = ask_rx.recv().await {
                let _ = ask.reply_tx.send(PermissionDecision::Allow);
            }
        });

        // First call — goes through ask channel.
        let result = broker.check("test-plugin", &cap).await;
        assert!(result.is_ok());

        // Second call — should be cached (no ask needed).
        let result = broker.check("test-plugin", &cap).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn permission_deny_is_not_cached() {
        let (broker, mut ask_rx) = PermissionBroker::new();
        let cap = Capability::Secret {
            key: "api_key".to_string(),
        };

        // Answer deny twice — because deny is not cached.
        tokio::spawn(async move {
            for _ in 0..2 {
                if let Some(ask) = ask_rx.recv().await {
                    let _ = ask.reply_tx.send(PermissionDecision::Deny);
                }
            }
        });

        let r1 = broker.check("test-plugin", &cap).await;
        assert!(r1.is_err());

        let r2 = broker.check("test-plugin", &cap).await;
        assert!(r2.is_err());
    }

    #[tokio::test]
    async fn invalidate_removes_grant() {
        let (broker, mut ask_rx) = PermissionBroker::new();
        let cap = Capability::Pty;

        tokio::spawn(async move {
            // Answer allow twice — once before invalidation, once after.
            for _ in 0..2 {
                if let Some(ask) = ask_rx.recv().await {
                    let _ = ask.reply_tx.send(PermissionDecision::Allow);
                }
            }
        });

        broker.check("p1", &cap).await.unwrap();

        let key = CapabilityKey::from(&cap);
        broker.invalidate("p1", &key);

        // After invalidation, should ask again (not use cache).
        broker.check("p1", &cap).await.unwrap();
    }
}
