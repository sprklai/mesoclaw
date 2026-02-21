//! Vercel AI Gateway model discovery implementation.
//!
//! Discovers models from Vercel AI Gateway using the /v1/models endpoint.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    DiscoveredModelInfo, DiscoveryError, ModelDiscovery, ProviderConfig, create_client,
    infer_context_limit, infer_cost_tier, infer_vision_support,
};
use crate::ai::providers::router::{ModelCapabilities, ModelModality};

/// Vercel AI Gateway model object
#[derive(Debug, Serialize, Deserialize)]
struct VercelGatewayModel {
    id: String,
    object: Option<String>,
    created: Option<i64>,
    owned_by: Option<String>,
    #[serde(default)]
    permission: Vec<serde_json::Value>,
}

/// Vercel AI Gateway models list response
#[derive(Debug, Serialize, Deserialize)]
struct VercelGatewayModelsResponse {
    data: Vec<VercelGatewayModel>,
}

/// Vercel AI Gateway model discovery implementation
pub struct VercelGatewayDiscovery;

impl VercelGatewayDiscovery {
    pub fn new() -> Self {
        Self
    }

    /// Check if a model should be excluded from discovery
    fn should_exclude(model_id: &str) -> bool {
        // Exclude embedding, audio, and image generation models
        let exclude_patterns = [
            "embedding",
            "whisper",
            "tts",
            "dall-e",
            "midjourney",
            "stable-diffusion",
        ];

        let lower = model_id.to_lowercase();
        exclude_patterns.iter().any(|p| lower.contains(p))
    }

    /// Extract the actual provider from a Vercel AI Gateway model ID
    fn extract_provider(model_id: &str) -> &str {
        // Vercel AI Gateway models are prefixed with provider, e.g. "openai/gpt-4o"
        if model_id.contains('/') {
            model_id.split('/').next().unwrap_or("unknown")
        } else {
            "unknown"
        }
    }
}

impl Default for VercelGatewayDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelDiscovery for VercelGatewayDiscovery {
    fn provider_id(&self) -> &str {
        "vercel-ai-gateway"
    }

    async fn discover_models(
        &self,
        config: &ProviderConfig,
    ) -> Result<Vec<DiscoveredModelInfo>, DiscoveryError> {
        let api_key = config.api_key.as_ref().ok_or_else(|| {
            DiscoveryError::MissingApiKey("Vercel AI Gateway API key required".to_string())
        })?;

        let client = create_client(config.timeout_secs)?;
        let url = self.discovery_url(config);

        log::info!(
            "[Vercel AI Gateway Discovery] Requesting models from: {}",
            url
        );

        let response = client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Vercel AI Gateway Discovery] HTTP request failed: {}", e);
                DiscoveryError::HttpError(e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscoveryError::ProviderUnavailable(format!(
                "Vercel AI Gateway returned status {}: {}",
                status, body
            )));
        }

        let vercel_response: VercelGatewayModelsResponse = response.json().await.map_err(|e| {
            DiscoveryError::ParseError(format!("Failed to parse Vercel AI Gateway response: {}", e))
        })?;

        log::info!(
            "[Vercel AI Gateway Discovery] Found {} model(s)",
            vercel_response.data.len()
        );

        let models = vercel_response
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

                // Create display name with provider prefix
                let display_name = if m.id.contains('/') {
                    m.id.clone()
                } else {
                    format!("{}/{}", Self::extract_provider(&m.id), m.id)
                };

                DiscoveredModelInfo {
                    model_id: m.id.clone(),
                    display_name: Some(display_name),
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
        format!("{}/v1/models", base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vercel_gateway_models_response_parsing() {
        let json = r#"{"data":[{"id":"openai/gpt-4o","object":"model","created":1234567890,"owned_by":"openai"}]}"#;

        let response: VercelGatewayModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "openai/gpt-4o");
    }

    #[test]
    fn test_vercel_gateway_should_exclude() {
        assert!(VercelGatewayDiscovery::should_exclude(
            "openai/text-embedding-ada-002"
        ));
        assert!(VercelGatewayDiscovery::should_exclude("dall-e-3"));
        assert!(!VercelGatewayDiscovery::should_exclude("openai/gpt-4o"));
        assert!(!VercelGatewayDiscovery::should_exclude(
            "anthropic/claude-sonnet-4"
        ));
    }

    #[test]
    fn test_vercel_gateway_extract_provider() {
        assert_eq!(
            VercelGatewayDiscovery::extract_provider("openai/gpt-4o"),
            "openai"
        );
        assert_eq!(
            VercelGatewayDiscovery::extract_provider("anthropic/claude-sonnet-4"),
            "anthropic"
        );
        assert_eq!(
            VercelGatewayDiscovery::extract_provider("gpt-4o"),
            "unknown"
        );
    }

    #[test]
    fn test_vercel_gateway_discovery_url() {
        let discovery = VercelGatewayDiscovery::new();
        let config = ProviderConfig::new("vercel-ai-gateway", "https://api.vercel.ai");
        assert_eq!(
            discovery.discovery_url(&config),
            "https://api.vercel.ai/v1/models"
        );
    }

    #[test]
    fn test_vercel_gateway_provider_id() {
        let discovery = VercelGatewayDiscovery::new();
        assert_eq!(discovery.provider_id(), "vercel-ai-gateway");
    }
}
