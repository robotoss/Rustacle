use serde::{Deserialize, Serialize};

/// A chat message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A request to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// Tool schema for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Streaming delta from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatDelta {
    /// Partial text content.
    Text { text: String },

    /// Start of a tool use block.
    ToolUseStart { id: String, name: String },

    /// Partial tool use arguments (JSON fragment).
    ToolUseDelta { id: String, delta: String },

    /// End of a tool use block — args are complete.
    ToolUseEnd { id: String },

    /// Token usage report.
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },

    /// Stream finished.
    Done,
}

/// Token cost tracking for a single request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenCost {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl TokenCost {
    /// Total tokens used.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// A named model profile that routes to a specific provider + model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Profile name (e.g., "default", "fast", "local").
    pub name: String,
    /// Provider identifier (e.g., "openai", "anthropic", "local").
    pub provider: String,
    /// Model name (e.g., "gpt-4o", "claude-sonnet-4-20250514", "llama3.1").
    pub model: String,
    /// API base URL (e.g., "<https://api.openai.com/v1>").
    pub api_base: Option<String>,
    /// Max tokens for this profile.
    pub max_tokens: Option<u32>,
    /// Temperature.
    pub temperature: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_cost_total() {
        let cost = TokenCost {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(cost.total(), 150);
    }

    #[test]
    fn chat_delta_serialization() {
        let delta = ChatDelta::Text {
            text: "hello".to_string(),
        };
        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains("\"type\":\"Text\""));
    }
}
