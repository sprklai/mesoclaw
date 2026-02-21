//! OpenAI model discovery implementation.
//!
//! Discovers models from OpenAI API using the /v1/models endpoint.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    DiscoveredModelInfo, DiscoveryError, ModelDiscovery, ProviderConfig, create_client,
    infer_context_limit, infer_cost_tier, infer_vision_support,
};
use crate::ai::providers::router::{ModelCapabilities, ModelModality};

/// OpenAI model object from /v1/models response
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIModel {
    id: String,
    object: String,
    created: Option<i64>,
    owned_by: Option<String>,
}

/// OpenAI models list response
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

/// OpenAI model discovery implementation
pub struct OpenAIDiscovery;

impl OpenAIDiscovery {
    pub fn new() -> Self {
        Self
    }

    /// Check if a model should be excluded from discovery
    fn should_exclude(model_id: &str) -> bool {
        // Exclude embedding, moderation, audio, and legacy models
        let exclude_patterns = [
            "embedding",
            "moderation",
            "whisper",
            "tts",
            "dall-e",
            "gpt-3.5-turbo-instruct",
            "babbage",
            "davinci",
            "curie",
            "ada",
        ];

        let lower = model_id.to_lowercase();
        exclude_patterns.iter().any(|p| lower.contains(p))
    }
}

impl Default for OpenAIDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelDiscovery for OpenAIDiscovery {
    fn provider_id(&self) -> &str {
        "openai"
    }

    async fn discover_models(
        &self,
        config: &ProviderConfig,
    ) -> Result<Vec<DiscoveredModelInfo>, DiscoveryError> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| DiscoveryError::MissingApiKey("OpenAI API key required".to_string()))?;

        let client = create_client(config.timeout_secs)?;
        let url = self.discovery_url(config);

        log::info!("[OpenAI Discovery] Requesting models from: {}", url);

        let response = client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|e| {
                log::error!("[OpenAI Discovery] HTTP request failed: {}", e);
                DiscoveryError::HttpError(e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscoveryError::ProviderUnavailable(format!(
                "OpenAI returned status {}: {}",
                status, body
            )));
        }

        let openai_response: OpenAIModelsResponse = response.json().await.map_err(|e| {
            DiscoveryError::ParseError(format!("Failed to parse OpenAI response: {}", e))
        })?;

        log::info!(
            "[OpenAI Discovery] Found {} model(s)",
            openai_response.data.len()
        );

        let models = openai_response
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
    fn test_openai_models_response_parsing() {
        let json = r#"{"data":[{"id":"gpt-4o","object":"model","created":1234567890,"owned_by":"openai"}]}"#;

        let response: OpenAIModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "gpt-4o");
    }

    #[test]
    fn test_openai_should_exclude() {
        assert!(OpenAIDiscovery::should_exclude("text-embedding-ada-002"));
        assert!(OpenAIDiscovery::should_exclude("whisper-1"));
        assert!(OpenAIDiscovery::should_exclude("dall-e-3"));
        assert!(!OpenAIDiscovery::should_exclude("gpt-4o"));
        assert!(!OpenAIDiscovery::should_exclude("gpt-4o-mini"));
    }

    #[test]
    fn test_openai_discovery_url() {
        let discovery = OpenAIDiscovery::new();
        let config = ProviderConfig::new("openai", "https://api.openai.com/v1");
        assert_eq!(
            discovery.discovery_url(&config),
            "https://api.openai.com/v1/models"
        );
    }

    #[test]
    fn test_openai_provider_id() {
        let discovery = OpenAIDiscovery::new();
        assert_eq!(discovery.provider_id(), "openai");
    }
}
