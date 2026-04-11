mod policy;
mod topics;

pub use policy::BackpressurePolicy;
pub use topics::{BusTopics, CwdChange, TerminalChunk};

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{broadcast, watch, RwLock};

/// A typed event bus with per-topic backpressure policies.
///
/// Plugins publish events; the UI and other plugins subscribe.
/// Each topic has a declared backpressure policy that determines
/// what happens when subscribers are slow.
pub struct Bus {
    /// Broadcast channels for `BlockPublisher` / `DropOldest` topics.
    broadcast_topics: RwLock<HashMap<String, broadcast::Sender<Bytes>>>,
    /// Watch channels for `CoalesceLatest` topics (only latest value kept).
    watch_topics: RwLock<HashMap<String, Arc<watch::Sender<Bytes>>>>,
    /// Topic policies
    policies: RwLock<HashMap<String, BackpressurePolicy>>,
}

impl Bus {
    /// Create a new event bus with default topic capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            broadcast_topics: RwLock::new(HashMap::new()),
            watch_topics: RwLock::new(HashMap::new()),
            policies: RwLock::new(HashMap::new()),
        }
    }

    /// Register a topic with a backpressure policy.
    pub async fn register_topic(&self, topic: &str, policy: BackpressurePolicy) {
        match policy {
            BackpressurePolicy::BlockPublisher | BackpressurePolicy::DropOldest => {
                let (tx, _) = broadcast::channel(256);
                self.broadcast_topics
                    .write()
                    .await
                    .insert(topic.to_string(), tx);
            }
            BackpressurePolicy::CoalesceLatest => {
                let (tx, _) = watch::channel(Bytes::new());
                self.watch_topics
                    .write()
                    .await
                    .insert(topic.to_string(), Arc::new(tx));
            }
        }
        self.policies
            .write()
            .await
            .insert(topic.to_string(), policy);
        tracing::debug!(topic, ?policy, "bus topic registered");
    }

    /// Publish an event to a topic.
    ///
    /// # Errors
    /// Returns an error if the topic is not registered.
    pub async fn publish(&self, topic: &str, payload: Bytes) -> Result<(), BusError> {
        let policies = self.policies.read().await;
        let policy = policies
            .get(topic)
            .ok_or_else(|| BusError::TopicNotFound(topic.to_string()))?;

        match policy {
            BackpressurePolicy::BlockPublisher | BackpressurePolicy::DropOldest => {
                let topics = self.broadcast_topics.read().await;
                let tx = topics
                    .get(topic)
                    .ok_or_else(|| BusError::TopicNotFound(topic.to_string()))?;
                // broadcast send fails only if there are no receivers — that's OK
                let _ = tx.send(payload);
            }
            BackpressurePolicy::CoalesceLatest => {
                let topics = self.watch_topics.read().await;
                let tx = topics
                    .get(topic)
                    .ok_or_else(|| BusError::TopicNotFound(topic.to_string()))?;
                tx.send_replace(payload);
            }
        }
        Ok(())
    }

    /// Subscribe to a broadcast topic. Returns an mpsc receiver.
    ///
    /// # Errors
    /// Returns an error if the topic is not registered or is `CoalesceLatest`.
    pub async fn subscribe_broadcast(
        &self,
        topic: &str,
    ) -> Result<broadcast::Receiver<Bytes>, BusError> {
        let topics = self.broadcast_topics.read().await;
        let tx = topics
            .get(topic)
            .ok_or_else(|| BusError::TopicNotFound(topic.to_string()))?;
        Ok(tx.subscribe())
    }

    /// Subscribe to a `CoalesceLatest` topic. Returns a watch receiver.
    ///
    /// # Errors
    /// Returns an error if the topic is not registered or is not `CoalesceLatest`.
    pub async fn subscribe_watch(
        &self,
        topic: &str,
    ) -> Result<watch::Receiver<Bytes>, BusError> {
        let topics = self.watch_topics.read().await;
        let tx = topics
            .get(topic)
            .ok_or_else(|| BusError::TopicNotFound(topic.to_string()))?;
        Ok(tx.subscribe())
    }

    /// Register all well-known terminal topics.
    pub async fn register_terminal_topics(&self) {
        self.register_topic(BusTopics::TERMINAL_OUTPUT, BackpressurePolicy::DropOldest)
            .await;
        self.register_topic(BusTopics::TERMINAL_CWD, BackpressurePolicy::CoalesceLatest)
            .await;
        self.register_topic(
            BusTopics::AGENT_REASONING,
            BackpressurePolicy::BlockPublisher,
        )
        .await;
        self.register_topic(BusTopics::AGENT_COST, BackpressurePolicy::CoalesceLatest)
            .await;
        self.register_topic(
            BusTopics::PERMISSION_ASK,
            BackpressurePolicy::BlockPublisher,
        )
        .await;
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

/// Event bus errors.
#[derive(thiserror::Error, Debug)]
pub enum BusError {
    #[error("topic not found: {0}")]
    TopicNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn broadcast_publish_subscribe() {
        let bus = Bus::new();
        bus.register_topic("test.broadcast", BackpressurePolicy::DropOldest)
            .await;

        let mut rx = bus.subscribe_broadcast("test.broadcast").await.unwrap();

        bus.publish("test.broadcast", Bytes::from_static(b"hello"))
            .await
            .unwrap();

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.as_ref(), b"hello");
    }

    #[tokio::test]
    async fn watch_coalesce_latest() {
        let bus = Bus::new();
        bus.register_topic("test.watch", BackpressurePolicy::CoalesceLatest)
            .await;

        let mut rx = bus.subscribe_watch("test.watch").await.unwrap();

        // Publish multiple values rapidly
        bus.publish("test.watch", Bytes::from_static(b"first"))
            .await
            .unwrap();
        bus.publish("test.watch", Bytes::from_static(b"second"))
            .await
            .unwrap();
        bus.publish("test.watch", Bytes::from_static(b"third"))
            .await
            .unwrap();

        // Watch should have the latest value
        rx.changed().await.unwrap();
        let val = rx.borrow().clone();
        assert_eq!(val.as_ref(), b"third");
    }

    #[tokio::test]
    async fn publish_to_unknown_topic_fails() {
        let bus = Bus::new();
        let result = bus
            .publish("nonexistent", Bytes::from_static(b"data"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn register_all_terminal_topics() {
        let bus = Bus::new();
        bus.register_terminal_topics().await;

        // Should be able to subscribe to all registered topics
        assert!(bus.subscribe_broadcast(BusTopics::TERMINAL_OUTPUT).await.is_ok());
        assert!(bus.subscribe_watch(BusTopics::TERMINAL_CWD).await.is_ok());
        assert!(
            bus.subscribe_broadcast(BusTopics::AGENT_REASONING)
                .await
                .is_ok()
        );
    }
}
