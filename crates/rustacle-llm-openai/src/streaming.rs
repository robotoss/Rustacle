use futures_util::StreamExt;
use rustacle_llm::provider::{ChatStream, LlmError};
use rustacle_llm::types::{ChatDelta, ChatRequest};
use tokio_util::sync::CancellationToken;

/// Stream chat completions from an OpenAI-compatible endpoint.
///
/// # Errors
/// Returns `LlmError::Provider` on network or API errors.
pub async fn stream_openai(
    client: &reqwest::Client,
    api_base: &str,
    api_key: Option<&str>,
    request: ChatRequest,
    cancel: CancellationToken,
) -> Result<ChatStream, LlmError> {
    let url = format!("{api_base}/chat/completions");

    let mut body = serde_json::to_value(&request).map_err(|e| LlmError::Provider {
        provider: "openai".to_string(),
        message: format!("serialize request: {e}"),
        retryable: false,
    })?;
    body["stream"] = serde_json::Value::Bool(true);

    let mut req = client.post(&url).json(&body);
    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }

    let resp = req.send().await.map_err(|e| LlmError::Provider {
        provider: "openai".to_string(),
        message: e.to_string(),
        retryable: e.is_timeout() || e.is_connect(),
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(LlmError::Provider {
            provider: "openai".to_string(),
            message: format!("HTTP {status}: {text}"),
            retryable: status.is_server_error(),
        });
    }

    let byte_stream = resp.bytes_stream();

    let stream = futures_util::stream::unfold(
        (byte_stream, cancel, String::new()),
        |(mut byte_stream, cancel, mut buffer)| async move {
            loop {
                if cancel.is_cancelled() {
                    return Some((Err(LlmError::Cancelled), (byte_stream, cancel, buffer)));
                }

                // Try to parse a complete SSE event from the buffer.
                if let Some(pos) = buffer.find("\n\n") {
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    if let Some(delta) = parse_sse_event(&event) {
                        if matches!(delta, ChatDelta::Done) {
                            return None;
                        }
                        return Some((Ok(delta), (byte_stream, cancel, buffer)));
                    }
                    continue;
                }

                // Read more bytes.
                match byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(LlmError::Provider {
                                provider: "openai".to_string(),
                                message: e.to_string(),
                                retryable: true,
                            }),
                            (byte_stream, cancel, buffer),
                        ));
                    }
                    None => return None,
                }
            }
        },
    );

    Ok(Box::pin(stream))
}

/// Parse a single SSE event line into a `ChatDelta`.
fn parse_sse_event(event: &str) -> Option<ChatDelta> {
    let data = event
        .lines()
        .find(|l| l.starts_with("data: "))
        .map(|l| &l[6..])?;

    if data == "[DONE]" {
        return Some(ChatDelta::Done);
    }

    let json: serde_json::Value = serde_json::from_str(data).ok()?;

    // Usage (may arrive without choices array)
    if let Some(usage) = json.get("usage") {
        if let (Some(input), Some(output)) = (
            usage["prompt_tokens"].as_u64(),
            usage["completion_tokens"].as_u64(),
        ) {
            return Some(ChatDelta::Usage {
                input_tokens: input,
                output_tokens: output,
            });
        }
    }

    let choice = json["choices"].get(0)?;
    let delta = &choice["delta"];

    // Text content
    if let Some(content) = delta["content"].as_str() {
        if !content.is_empty() {
            return Some(ChatDelta::Text {
                text: content.to_string(),
            });
        }
    }

    // Tool calls
    if let Some(tool_calls) = delta["tool_calls"].as_array() {
        for tc in tool_calls {
            let id = tc["id"].as_str().unwrap_or("").to_string();
            let idx = tc["index"].as_u64().unwrap_or(0);

            if let Some(func) = tc.get("function") {
                if let Some(name) = func["name"].as_str() {
                    return Some(ChatDelta::ToolUseStart {
                        id: if id.is_empty() {
                            format!("call_{idx}")
                        } else {
                            id
                        },
                        name: name.to_string(),
                    });
                }
                if let Some(args) = func["arguments"].as_str() {
                    return Some(ChatDelta::ToolUseDelta {
                        id: if id.is_empty() {
                            format!("call_{idx}")
                        } else {
                            id
                        },
                        delta: args.to_string(),
                    });
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_delta() {
        let event = "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}";
        let delta = parse_sse_event(event).unwrap();
        assert!(matches!(delta, ChatDelta::Text { text } if text == "Hello"));
    }

    #[test]
    fn parse_done() {
        let event = "data: [DONE]";
        let delta = parse_sse_event(event).unwrap();
        assert!(matches!(delta, ChatDelta::Done));
    }

    #[test]
    fn parse_tool_call_start() {
        let event = r#"data: {"choices":[{"delta":{"tool_calls":[{"id":"call_1","index":0,"function":{"name":"fs_read"}}]}}]}"#;
        let delta = parse_sse_event(event).unwrap();
        assert!(matches!(delta, ChatDelta::ToolUseStart { name, .. } if name == "fs_read"));
    }

    #[test]
    fn parse_usage() {
        let event = r#"data: {"usage":{"prompt_tokens":100,"completion_tokens":50}}"#;
        let delta = parse_sse_event(event).unwrap();
        assert!(matches!(
            delta,
            ChatDelta::Usage {
                input_tokens: 100,
                output_tokens: 50
            }
        ));
    }
}
