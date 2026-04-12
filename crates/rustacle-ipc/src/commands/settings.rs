use serde::{Deserialize, Serialize};

/// A single setting entry.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SettingEntry {
    pub key: String,
    pub value: serde_json::Value,
}

/// Request for `get_setting`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GetSettingRequest {
    pub key: String,
}

/// Response from `get_setting`. Value is a JSON string.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct GetSettingResponse {
    pub key: String,
    /// JSON-encoded value string.
    pub value_json: String,
}

/// Request for `set_setting`. Value is a JSON string.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SetSettingRequest {
    pub key: String,
    /// JSON-encoded value string.
    pub value_json: String,
}

/// Request for `test_model_connection`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TestModelRequest {
    pub provider: String,
    pub model: String,
    pub api_base: String,
    pub api_key: String,
}

/// Response from `test_model_connection`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TestModelResponse {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
}
