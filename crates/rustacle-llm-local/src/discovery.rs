use std::time::Duration;

use serde::Deserialize;

/// A discovered local LLM server.
#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    pub name: String,
    pub api_base: String,
    pub models: Vec<String>,
}

/// Well-known local server endpoints to probe.
const PROBES: &[(&str, &str)] = &[
    ("Ollama", "http://127.0.0.1:11434/v1"),
    ("LM Studio", "http://127.0.0.1:1234/v1"),
    ("llama.cpp", "http://127.0.0.1:8080/v1"),
    ("vLLM", "http://127.0.0.1:8000/v1"),
];

/// Probe localhost for running LLM servers.
///
/// Returns all servers that respond within the timeout.
pub async fn discover_local_servers() -> Vec<DiscoveredServer> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    let mut servers = Vec::new();

    for (name, api_base) in PROBES {
        let url = format!("{api_base}/models");
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let models = resp
                    .json::<ModelsResponse>()
                    .await
                    .map(|r| r.data.into_iter().map(|m| m.id).collect())
                    .unwrap_or_default();

                tracing::info!(server = name, api_base, models = ?models, "local LLM server found");

                servers.push(DiscoveredServer {
                    name: (*name).to_string(),
                    api_base: (*api_base).to_string(),
                    models,
                });
            }
            _ => {
                tracing::debug!(server = name, api_base, "local server not available");
            }
        }
    }

    servers
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_list_is_not_empty() {
        assert!(!PROBES.is_empty());
    }
}
