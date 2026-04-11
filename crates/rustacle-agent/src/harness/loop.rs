//! The main ReAct-style thinking loop.
//!
//! Cycle: assemble prompt -> stream LLM -> parse deltas -> dispatch tools -> repeat.

use std::sync::Arc;
use std::time::Instant;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use rustacle_llm::types::{ChatDelta, ChatMessage, ChatRequest, Role};
use rustacle_llm::LlmProvider;

use crate::prompt::assemble_prompt;
use crate::tools::ToolDispatchTable;
use crate::turn_context::TurnContext;

use super::cancel::CancelHandle;
use super::streaming::{FlushConfig, ThoughtBuffer};
use super::{HarnessError, ReasoningStep, StepKind, TurnBudget, TurnCost};

/// Parsed tool call accumulated from streaming deltas.
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    pub id: String,
    pub name: String,
    pub args_json: String,
}

/// The agent harness drives the thinking loop.
pub struct Harness {
    /// LLM provider for streaming.
    provider: Arc<dyn LlmProvider>,
    /// Tool dispatch table.
    tools: Arc<ToolDispatchTable>,
    /// Channel for emitting reasoning steps to the UI.
    step_tx: mpsc::UnboundedSender<ReasoningStep>,
    /// Flush config for partial thoughts.
    flush_config: FlushConfig,
    /// Budget limits.
    budget: TurnBudget,
}

impl Harness {
    /// Create a new harness.
    #[must_use]
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolDispatchTable>,
        step_tx: mpsc::UnboundedSender<ReasoningStep>,
    ) -> Self {
        Self {
            provider,
            tools,
            step_tx,
            flush_config: FlushConfig::default(),
            budget: TurnBudget::default(),
        }
    }

    /// Set flush config.
    #[must_use]
    pub fn with_flush_config(mut self, config: FlushConfig) -> Self {
        self.flush_config = config;
        self
    }

    /// Set budget.
    #[must_use]
    pub fn with_budget(mut self, budget: TurnBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Run a turn. Returns the final answer text, or an error.
    ///
    /// # Errors
    /// Returns `HarnessError` on cancellation, budget exceeded, or LLM/tool errors.
    #[allow(clippy::too_many_lines)]
    pub async fn run_turn(
        &self,
        ctx: TurnContext,
        cancel: CancelHandle,
    ) -> Result<String, HarnessError> {
        let start = Instant::now();
        let mut cost = TurnCost::default();
        let turn_id = ctx.turn_id.clone();

        let mut conversation: Vec<ChatMessage> = Vec::new();
        conversation.push(ChatMessage {
            role: Role::User,
            content: ctx.user_turn.text.clone(),
            tool_call_id: None,
            name: None,
        });

        loop {
            if cancel.is_cancelled() {
                return Err(HarnessError::Cancelled);
            }

            cost.elapsed_ms = start.elapsed().as_secs() * 1000 + u64::from(start.elapsed().subsec_millis());
            if let Some(reason) = cost.exceeds(&self.budget) {
                self.emit_step(&turn_id, StepKind::Error {
                    message: reason.to_owned(),
                    retryable: false,
                });
                return Err(HarnessError::BudgetExceeded(reason.to_owned()));
            }

            let prompt = assemble_prompt(&ctx);
            let request = Self::build_request(&ctx, &prompt, &conversation);

            let stream = self
                .stream_with_retry(&request, &cancel, &mut cost)
                .await?;

            let (answer_text, tool_calls) =
                self.consume_stream(stream, &cancel, &turn_id, &mut cost)
                    .await?;

            if tool_calls.is_empty() {
                self.emit_step(&turn_id, StepKind::Answer {
                    text: answer_text.clone(),
                });
                info!(turn_id = %turn_id, tokens = cost.total_tokens(), "turn complete");
                return Ok(answer_text);
            }

            conversation.push(ChatMessage {
                role: Role::Assistant,
                content: answer_text,
                tool_call_id: None,
                name: None,
            });

            for call in &tool_calls {
                cost.tool_calls += 1;
                let args: serde_json::Value =
                    serde_json::from_str(&call.args_json).unwrap_or_default();

                self.emit_step(&turn_id, StepKind::ToolCall {
                    tool: call.name.clone(),
                    args: args.clone(),
                    tab_target: None,
                });

                let tool_start = Instant::now();
                let result = self
                    .tools
                    .dispatch(&call.name, args, cancel.child())
                    .await;
                #[allow(clippy::cast_possible_truncation)]
                let duration_ms = tool_start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;

                let (ok, summary) = match result {
                    Ok(output) => (true, output),
                    Err(e) => (false, e),
                };

                self.emit_step(&turn_id, StepKind::ToolResult {
                    tool: call.name.clone(),
                    ok,
                    summary: summary.clone(),
                    duration_ms,
                });

                conversation.push(ChatMessage {
                    role: Role::Tool,
                    content: summary,
                    tool_call_id: Some(call.id.clone()),
                    name: Some(call.name.clone()),
                });
            }

            debug!(
                turn_id = %turn_id,
                tool_calls = tool_calls.len(),
                "loop iteration complete, continuing"
            );
        }
    }

    /// Build the `ChatRequest` from context, prompt, and conversation so far.
    fn build_request(
        ctx: &TurnContext,
        prompt: &crate::prompt::Prompt,
        conversation: &[ChatMessage],
    ) -> ChatRequest {
        let mut messages = vec![ChatMessage {
            role: Role::System,
            content: prompt.to_system_message(),
            tool_call_id: None,
            name: None,
        }];

        for msg in &ctx.history.messages {
            messages.push(ChatMessage {
                role: match msg.role {
                    crate::turn_context::HistoryRole::User => Role::User,
                    crate::turn_context::HistoryRole::Assistant => Role::Assistant,
                    crate::turn_context::HistoryRole::Tool => Role::Tool,
                },
                content: msg.content.clone(),
                tool_call_id: None,
                name: None,
            });
        }

        messages.extend_from_slice(conversation);

        ChatRequest {
            model: ctx.model_profile.model.clone(),
            messages,
            tools: prompt.tool_schemas().to_vec(),
            max_tokens: ctx.model_profile.max_tokens,
            temperature: ctx.model_profile.temperature,
        }
    }

    /// Consume the LLM stream, returning answer text and parsed tool calls.
    async fn consume_stream(
        &self,
        stream: rustacle_llm::provider::ChatStream,
        cancel: &CancelHandle,
        turn_id: &str,
        cost: &mut TurnCost,
    ) -> Result<(String, Vec<ParsedToolCall>), HarnessError> {
        let mut thought_buf = ThoughtBuffer::new(self.flush_config.clone());
        let mut tool_calls: Vec<ParsedToolCall> = Vec::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();
        let mut answer_text = String::new();

        tokio::pin!(stream);

        loop {
            let delta = tokio::select! {
                d = stream.next() => d,
                () = cancel.token().cancelled() => {
                    return Err(HarnessError::Cancelled);
                }
            };

            let Some(delta_result) = delta else { break };

            let delta = match delta_result {
                Ok(d) => d,
                Err(e) => {
                    warn!(error = %e, "LLM stream error");
                    break;
                }
            };

            match delta {
                ChatDelta::Text { text } => {
                    answer_text.push_str(&text);
                    thought_buf.push(&text);
                    if thought_buf.should_flush_sentence() {
                        let chunk = thought_buf.take();
                        self.emit_step(turn_id, StepKind::Thought {
                            text: chunk,
                            partial: true,
                        });
                    }
                }
                ChatDelta::ToolUseStart { id, name } => {
                    if !thought_buf.is_empty() {
                        let chunk = thought_buf.take();
                        self.emit_step(turn_id, StepKind::Thought {
                            text: chunk,
                            partial: true,
                        });
                    }
                    current_tool_id = id;
                    current_tool_name = name;
                    current_tool_args.clear();
                }
                ChatDelta::ToolUseDelta { delta, .. } => {
                    current_tool_args.push_str(&delta);
                }
                ChatDelta::ToolUseEnd { .. } => {
                    tool_calls.push(ParsedToolCall {
                        id: current_tool_id.clone(),
                        name: current_tool_name.clone(),
                        args_json: current_tool_args.clone(),
                    });
                    current_tool_id.clear();
                    current_tool_name.clear();
                    current_tool_args.clear();
                }
                ChatDelta::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    cost.input_tokens += input_tokens;
                    cost.output_tokens += output_tokens;
                }
                ChatDelta::Done => break,
            }
        }

        if !thought_buf.is_empty() {
            let chunk = thought_buf.take();
            self.emit_step(turn_id, StepKind::Thought {
                text: chunk,
                partial: false,
            });
        }

        Ok((answer_text, tool_calls))
    }

    /// Stream with retry for transport errors (max 3 attempts).
    async fn stream_with_retry(
        &self,
        request: &ChatRequest,
        cancel: &CancelHandle,
        _cost: &mut TurnCost,
    ) -> Result<rustacle_llm::provider::ChatStream, HarnessError> {
        let max_retries = 3u32;
        let mut attempt = 0u32;

        loop {
            attempt += 1;

            let result = tokio::select! {
                r = self.provider.stream(request.clone(), cancel.child()) => r,
                () = cancel.token().cancelled() => {
                    return Err(HarnessError::Cancelled);
                }
            };

            match result {
                Ok(stream) => return Ok(stream),
                Err(rustacle_llm::provider::LlmError::Cancelled) => {
                    return Err(HarnessError::Cancelled);
                }
                Err(e) if attempt < max_retries => {
                    let retryable = matches!(
                        &e,
                        rustacle_llm::provider::LlmError::Provider { retryable: true, .. }
                    );
                    if retryable {
                        let backoff_ms = 500 * u64::from(2u32.pow(attempt - 1));
                        warn!(attempt, backoff_ms, error = %e, "retryable LLM error");
                        tokio::select! {
                            () = tokio::time::sleep(
                                std::time::Duration::from_millis(backoff_ms)
                            ) => {}
                            () = cancel.token().cancelled() => {
                                return Err(HarnessError::Cancelled);
                            }
                        }
                        continue;
                    }
                    return Err(HarnessError::Llm(e));
                }
                Err(e) => return Err(HarnessError::Llm(e)),
            }
        }
    }

    /// Emit a reasoning step.
    fn emit_step(&self, turn_id: &str, kind: StepKind) {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() * 1000 + u64::from(d.subsec_millis()));

        let step = ReasoningStep {
            id: ulid::Ulid::new().to_string(),
            parent_id: None,
            turn_id: turn_id.to_owned(),
            ts_ms,
            kind,
        };
        if self.step_tx.send(step).is_err() {
            warn!("step receiver dropped");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsed_tool_call_json_roundtrip() {
        let call = ParsedToolCall {
            id: "tc1".into(),
            name: "fs_read".into(),
            args_json: r#"{"path":"src/main.rs"}"#.into(),
        };
        let args: serde_json::Value = serde_json::from_str(&call.args_json).unwrap();
        assert_eq!(args["path"], "src/main.rs");
    }

    #[test]
    fn budget_check() {
        let budget = TurnBudget {
            max_tool_calls: 5,
            max_duration_ms: 1000,
            max_tokens: 100,
        };

        let mut cost = TurnCost::default();
        assert!(cost.exceeds(&budget).is_none());

        cost.tool_calls = 5;
        assert_eq!(cost.exceeds(&budget), Some("max tool calls exceeded"));

        cost.tool_calls = 0;
        cost.input_tokens = 60;
        cost.output_tokens = 50;
        assert_eq!(cost.exceeds(&budget), Some("max tokens exceeded"));
    }
}
