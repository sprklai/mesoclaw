//! Ollama model discovery implementation.
//!
//! Discovers models from local Ollama instance using the native /api/tags endpoint.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    DiscoveredModelInfo, DiscoveryError, ModelDiscovery, ProviderConfig, create_client,
    infer_context_limit, infer_vision_support,
};
use crate::ai::providers::router::{CostTier, ModelCapabilities, ModelModality};

/// Ollama model response from GET /api/tags
#[derive(Debug, Serialize, Deserialize)]
struct OllamaModel {
    name: String,
    modified_at: Option<String>,
    size: Option<i64>,
    digest: Option<String>,
}

/// Ollama models response
#[derive(Debug, Serialize, Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

/// Ollama model discovery implementation
pub struct OllamaDiscovery;

impl OllamaDiscovery {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OllamaDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelDiscovery for OllamaDiscovery {
    fn provider_id(&self) -> &str {
        "ollama"
    }

    async fn discover_models(
        &self,
        config: &ProviderConfig,
    ) -> Result<Vec<DiscoveredModelInfo>, DiscoveryError> {
        let client = create_client(config.timeout_secs)?;
        let url = self.discovery_url(config);

        log::info!("[Ollama Discovery] Requesting models from: {}", url);

        let response = client.get(&url).send().await.map_err(|e| {
            log::error!("[Ollama Discovery] HTTP request failed: {}", e);
            DiscoveryError::ProviderUnavailable(format!(
                "Failed to connect to Ollama at {}: {}",
                config.base_url, e
            ))
        })?;

        if !response.status().is_success() {
            return Err(DiscoveryError::ProviderUnavailable(format!(
                "Ollama returned status: {}",
                response.status()
            )));
        }

        let ollama_response: OllamaModelsResponse = response.json().await.map_err(|e| {
            DiscoveryError::ParseError(format!("Failed to parse Ollama response: {}", e))
        })?;

        log::info!(
            "[Ollama Discovery] Found {} model(s)",
            ollama_response.models.len()
        );

        let models = ollama_response
            .models
            .into_iter()
            .map(|m| {
                let supports_vision = infer_vision_support(&m.name);
                let modalities = if supports_vision {
                    vec![ModelModality::Text, ModelModality::Image]
                } else {
                    vec![ModelModality::Text]
                };

                let capabilities = if supports_vision {
                    ModelCapabilities::full_featured()
                } else {
                    ModelCapabilities::text_only()
                };

                // Ollama size is in bytes, convert to rough context limit estimate
                // or use heuristic-based inference
                let context_limit = m.size.and_then(|s| {
                    // Very rough estimate: 1B params ≈ 4GB ≈ 8K context
                    // This is just a heuristic
                    let params_estimate = s / 4_000_000_000; // Rough params in billions
                    if params_estimate > 0 {
                        Some((params_estimate * 4000).min(128_000) as i32)
                    } else {
                        infer_context_limit(&m.name)
                    }
                });

                DiscoveredModelInfo {
                    model_id: m.name.clone(),
                    display_name: Some(m.name),
                    provider_id: self.provider_id().to_string(),
                    cost_tier: CostTier::Low, // Ollama models are local/free
                    context_limit,
                    modalities,
                    capabilities: Some(capabilities),
                    supports_vision,
                }
            })
            .collect();

        Ok(models)
    }

    async fn is_available(&self, config: &ProviderConfig) -> bool {
        let client = match create_client(5) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let url = self.discovery_url(config);

        match client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    fn discovery_url(&self, config: &ProviderConfig) -> String {
        // Handle both native Ollama URL and OpenAI-compatible URL
        let base = config
            .base_url
            .trim_end_matches("/v1")
            .trim_end_matches('/');
        format!("{}/api/tags", base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_models_response_parsing() {
        let json = r#"{"models":[{"name":"llama3","modified_at":"2024-01-01T00:00:00Z","size":4000000000}]}"#;

        let response: OllamaModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].name, "llama3");
    }

    #[test]
    fn test_ollama_discovery_url() {
        let discovery = OllamaDiscovery::new();

        // Native URL
        let config = ProviderConfig::new("ollama", "http://localhost:11434");
        assert_eq!(
            discovery.discovery_url(&config),
            "http://localhost:11434/api/tags"
        );

        // OpenAI-compatible URL
        let config = ProviderConfig::new("ollama", "http://localhost:11434/v1");
        assert_eq!(
            discovery.discovery_url(&config),
            "http://localhost:11434/api/tags"
        );
    }

    #[test]
    fn test_ollama_provider_id() {
        let discovery = OllamaDiscovery::new();
        assert_eq!(discovery.provider_id(), "ollama");
    }
}
