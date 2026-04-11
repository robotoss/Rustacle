//! Tool dispatch table: register, lookup, partition, and fan-out.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use super::{Concurrency, Tool, ToolCtx, ToolError, placeholder_dispatch};

/// Registry of all available tools, keyed by name (sorted for determinism).
pub struct ToolDispatchTable {
    by_name: BTreeMap<String, Arc<dyn Tool>>,
    /// Working directory for tool execution.
    cwd: std::path::PathBuf,
}

impl ToolDispatchTable {
    /// Create a new empty dispatch table.
    #[must_use]
    pub fn new(cwd: std::path::PathBuf) -> Self {
        Self {
            by_name: BTreeMap::new(),
            cwd,
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.schema().name.clone();
        debug!(tool = %name, "registered tool");
        self.by_name.insert(name, tool);
    }

    /// Look up a tool by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.by_name.get(name)
    }

    /// List all registered tool names.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.by_name.keys().map(String::as_str).collect()
    }

    /// Dispatch a single tool call. Returns `Ok(summary)` or `Err(error message)`.
    ///
    /// If the tool is not registered, falls back to a placeholder.
    ///
    /// # Errors
    /// Returns `Err` with a human-readable error string on validation or execution failure.
    pub async fn dispatch(
        &self,
        name: &str,
        args: serde_json::Value,
        cancel: CancellationToken,
    ) -> Result<String, String> {
        let Some(tool) = self.by_name.get(name) else {
            return Ok(placeholder_dispatch(name, &args));
        };

        // Validate
        if let Err(e) = tool.validate(&args) {
            return Err(format!("validation error: {e}"));
        }

        let ctx = ToolCtx {
            cancel: cancel.clone(),
            cwd: self.cwd.clone(),
        };

        // Execute with cancellation
        let result = tokio::select! {
            r = tool.call(args, ctx) => r,
            () = cancel.cancelled() => Err(ToolError::Cancelled),
        };

        match result {
            Ok(output) => Ok(output.summary),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Dispatch multiple tool calls, partitioning into concurrent and serialized.
    ///
    /// Concurrent tools are fanned out via `JoinSet`; serialized run one at a time.
    pub async fn dispatch_batch(
        &self,
        calls: Vec<(String, String, serde_json::Value)>, // (id, name, args)
        parent_cancel: CancellationToken,
    ) -> Vec<(String, Result<String, String>)> {
        let (concurrent, serialized): (Vec<_>, Vec<_>) = calls
            .into_iter()
            .partition(|(_, name, _)| {
                self.by_name
                    .get(name.as_str())
                    .is_some_and(|t| t.concurrency() == Concurrency::Concurrent)
            });

        let mut results = Vec::new();

        // Fan out concurrent tools
        if !concurrent.is_empty() {
            let mut set = JoinSet::new();
            for (id, name, args) in concurrent {
                let tool = self.by_name.get(&name).cloned();
                let child_cancel = parent_cancel.child_token();
                let cwd = self.cwd.clone();

                set.spawn(async move {
                    let Some(tool) = tool else {
                        return (id, Ok(placeholder_dispatch(&name, &args)));
                    };

                    if let Err(e) = tool.validate(&args) {
                        return (id, Err(format!("validation error: {e}")));
                    }

                    let ctx = ToolCtx {
                        cancel: child_cancel.clone(),
                        cwd,
                    };

                    let result = tokio::select! {
                        r = tool.call(args, ctx) => r,
                        () = child_cancel.cancelled() => Err(ToolError::Cancelled),
                    };

                    let r = match result {
                        Ok(output) => Ok(output.summary),
                        Err(e) => Err(e.to_string()),
                    };
                    (id, r)
                });
            }

            while let Some(join_result) = set.join_next().await {
                match join_result {
                    Ok(r) => results.push(r),
                    Err(e) => {
                        warn!(error = %e, "tool task panicked");
                    }
                }
            }
        }

        // Serialized tools one at a time
        for (id, name, args) in serialized {
            if parent_cancel.is_cancelled() {
                results.push((id, Err("cancelled".to_owned())));
                continue;
            }

            let child_cancel = parent_cancel.child_token();
            let result = self.dispatch(&name, args, child_cancel).await;
            results.push((id, result));
        }

        // Sort by id for deterministic output order
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{Capability, Concurrency, Tool, ToolCtx, ToolError, ToolOutput};
    use rustacle_llm::types::ToolSchema;
    use serde_json::Value;

    struct EchoTool;

    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn schema(&self) -> ToolSchema {
            ToolSchema {
                name: "echo".to_owned(),
                description: "Echo input".to_owned(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            }
        }

        fn validate(&self, _args: &Value) -> Result<(), ToolError> {
            Ok(())
        }

        fn concurrency(&self) -> Concurrency {
            Concurrency::Concurrent
        }

        fn required_capabilities(&self, _args: &Value) -> Vec<Capability> {
            vec![]
        }

        async fn call(&self, args: Value, _ctx: ToolCtx) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput {
                summary: format!("echo: {args}"),
                payload: None,
            })
        }
    }

    #[tokio::test]
    async fn dispatch_registered_tool() {
        let mut table = ToolDispatchTable::new(std::path::PathBuf::from("/tmp"));
        table.register(Arc::new(EchoTool));

        let cancel = CancellationToken::new();
        let result = table
            .dispatch("echo", serde_json::json!({"msg": "hi"}), cancel)
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().contains("echo"));
    }

    #[tokio::test]
    async fn dispatch_unknown_tool_uses_placeholder() {
        let table = ToolDispatchTable::new(std::path::PathBuf::from("/tmp"));
        let cancel = CancellationToken::new();
        let result = table
            .dispatch("nonexistent", serde_json::json!({}), cancel)
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().contains("[placeholder]"));
    }

    #[test]
    fn names_sorted() {
        let mut table = ToolDispatchTable::new(std::path::PathBuf::from("/tmp"));
        table.register(Arc::new(EchoTool));
        let names = table.names();
        assert_eq!(names, vec!["echo"]);
    }
}
