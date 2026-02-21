//! Groq model discovery implementation.
//!
//! Discovers models from Groq API using the OpenAI-compatible /openai/v1/models endpoint.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    DiscoveredModelInfo, DiscoveryError, ModelDiscovery, ProviderConfig, create_client,
    infer_context_limit, infer_cost_tier, infer_vision_support,
};
use crate::ai::providers::router::{ModelCapabilities, ModelModality};

/// Groq/OpenAI-compatible model object
#[derive(Debug, Serialize, Deserialize)]
struct GroqModel {
    id: String,
    object: Option<String>,
    created: Option<i64>,
    owned_by: Option<String>,
}

/// Groq models list response
#[derive(Debug, Serialize, Deserialize)]
struct GroqModelsResponse {
    data: Vec<GroqModel>,
}

/// Groq model discovery implementation
pub struct GroqDiscovery;

impl GroqDiscovery {
    pub fn new() -> Self {
        Self
    }

    /// Check if a model should be excluded from discovery
    fn should_exclude(model_id: &str) -> bool {
        // Exclude embedding and audio models
        let exclude_patterns = ["embed", "whisper", "tts"];

        let lower = model_id.to_lowercase();
        exclude_patterns.iter().any(|p| lower.contains(p))
    }
}

impl Default for GroqDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelDiscovery for GroqDiscovery {
    fn provider_id(&self) -> &str {
        "groq"
    }

    async fn discover_models(
        &self,
        config: &ProviderConfig,
    ) -> Result<Vec<DiscoveredModelInfo>, DiscoveryError> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| DiscoveryError::MissingApiKey("Groq API key required".to_string()))?;

        let client = create_client(config.timeout_secs)?;
        let url = self.discovery_url(config);

        log::info!("[Groq Discovery] Requesting models from: {}", url);

        let response = client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Groq Discovery] HTTP request failed: {}", e);
                DiscoveryError::HttpError(e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscoveryError::ProviderUnavailable(format!(
                "Groq returned status {}: {}",
                status, body
            )));
        }

        let groq_response: GroqModelsResponse = response.json().await.map_err(|e| {
            DiscoveryError::ParseError(format!("Failed to parse Groq response: {}", e))
        })?;

        log::info!(
            "[Groq Discovery] Found {} model(s)",
            groq_response.data.len()
        );

        let models = groq_response
            .data
            .into_iter()
            .filter(|m| !Self::should_exclude(&m.id))
            .map(|m| {
                let supports_vision = infer_vision_support(&m.id);
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

                DiscoveredModelInfo {
                    model_id: m.id.clone(),
                    display_name: Some(m.id.clone()),
                    provider_id: self.provider_id().to_string(),
                    cost_tier: infer_cost_tier(&m.id),
                    context_limit: infer_context_limit(&m.id),
                    modalities,
                    capabilities: Some(capabilities),
                    supports_vision,
                }
            })
            .collect();

        Ok(models)
    }

    async fn is_available(&self, config: &ProviderConfig) -> bool {
        if config.api_key.is_none() {
            return false;
        }

        let client = match create_client(5) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let url = self.discovery_url(config);
        let api_key = config.api_key.as_ref().unwrap();

        match client.get(&url).bearer_auth(api_key).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    fn discovery_url(&self, config: &ProviderConfig) -> String {
        let base = config.base_url.trim_end_matches('/');
        format!("{}/models", base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_models_response_parsing() {
        let json = r#"{"data":[{"id":"llama-3.1-70b-versatile","object":"model","created":1234567890,"owned_by":"meta"}]}"#;

        let response: GroqModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "llama-3.1-70b-versatile");
    }

    #[test]
    fn test_groq_should_exclude() {
        assert!(GroqDiscovery::should_exclude("nomic-embed-text"));
        assert!(GroqDiscovery::should_exclude("whisper-large-v3"));
        assert!(!GroqDiscovery::should_exclude("llama-3.1-70b-versatile"));
        assert!(!GroqDiscovery::should_exclude("mixtral-8x7b-32768"));
    }

    #[test]
    fn test_groq_discovery_url() {
        let discovery = GroqDiscovery::new();
        let config = ProviderConfig::new("groq", "https://api.groq.com/openai/v1");
        assert_eq!(
            discovery.discovery_url(&config),
            "https://api.groq.com/openai/v1/models"
        );
    }

    #[test]
    fn test_groq_provider_id() {
        let discovery = GroqDiscovery::new();
        assert_eq!(discovery.provider_id(), "groq");
    }
}
