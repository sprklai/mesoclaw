//! OpenRouter model discovery implementation.
//!
//! Discovers models from OpenRouter using the /api/v1/models endpoint.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    DiscoveredModelInfo, DiscoveryError, ModelDiscovery, ProviderConfig, create_client,
    infer_context_limit, infer_cost_tier, infer_vision_support,
};
use crate::ai::providers::router::{CostTier, ModelCapabilities, ModelModality};

/// OpenRouter model pricing
#[derive(Debug, Serialize, Deserialize, Default)]
struct OpenRouterPricing {
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    completion: Option<String>,
    #[serde(default)]
    image: Option<String>,
}

/// OpenRouter model object
#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    description: Option<String>,
    context_length: Option<i32>,
    pricing: Option<OpenRouterPricing>,
    #[serde(default)]
    top_provider: Option<serde_json::Value>,
    #[serde(default)]
    architecture: Option<OpenRouterArchitecture>,
}

/// OpenRouter model architecture info
#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterArchitecture {
    modality: Option<String>,
    #[serde(default)]
    tokenizer: Option<String>,
    #[serde(default)]
    instruct_type: Option<String>,
}

/// OpenRouter models list response
#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

/// OpenRouter model discovery implementation
pub struct OpenRouterDiscovery;

impl OpenRouterDiscovery {
    pub fn new() -> Self {
        Self
    }

    /// Check if a model should be excluded from discovery
    fn should_exclude(model_id: &str) -> bool {
        // Exclude embedding and image generation models
        let exclude_patterns = ["embedding", "dall-e", "midjourney", "stable-diffusion"];

        let lower = model_id.to_lowercase();
        exclude_patterns.iter().any(|p| lower.contains(p))
    }

    /// Parse modality string from OpenRouter
    fn parse_modality(modality_str: Option<&str>) -> Vec<ModelModality> {
        match modality_str {
            Some(s) => {
                let lower = s.to_lowercase();
                let mut modalities = vec![ModelModality::Text];

                if lower.contains("image") || lower.contains("vision") {
                    modalities.push(ModelModality::Image);
                }
                if lower.contains("audio") {
                    modalities.push(ModelModality::AudioTranscription);
                }

                modalities
            }
            None => vec![ModelModality::Text],
        }
    }

    /// Infer cost tier from pricing if available
    fn infer_cost_tier_from_pricing(pricing: Option<&OpenRouterPricing>) -> CostTier {
        if let Some(pricing) = pricing {
            if let Some(prompt) = &pricing.prompt {
                // Parse price per token (format: "0.000001" or similar)
                if let Ok(price) = prompt.parse::<f64>() {
                    if price == 0.0 {
                        return CostTier::Low;
                    } else if price < 0.00001 {
                        return CostTier::Low;
                    } else if price < 0.0001 {
                        return CostTier::Medium;
                    } else {
                        return CostTier::High;
                    }
                }
            }
        }
        CostTier::Medium
    }
}

impl Default for OpenRouterDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelDiscovery for OpenRouterDiscovery {
    fn provider_id(&self) -> &str {
        "openrouter"
    }

    async fn discover_models(
        &self,
        config: &ProviderConfig,
    ) -> Result<Vec<DiscoveredModelInfo>, DiscoveryError> {
        let api_key = config.api_key.as_ref().ok_or_else(|| {
            DiscoveryError::MissingApiKey("OpenRouter API key required".to_string())
        })?;

        let client = create_client(config.timeout_secs)?;
        let url = self.discovery_url(config);

        log::info!("[OpenRouter Discovery] Requesting models from: {}", url);

        let response = client
            .get(&url)
            .bearer_auth(api_key)
            .header("HTTP-Referer", "https://mesoclaw.app")
            .header("X-Title", "MesoClaw")
            .send()
            .await
            .map_err(|e| {
                log::error!("[OpenRouter Discovery] HTTP request failed: {}", e);
                DiscoveryError::HttpError(e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DiscoveryError::ProviderUnavailable(format!(
                "OpenRouter returned status {}: {}",
                status, body
            )));
        }

        let openrouter_response: OpenRouterModelsResponse = response.json().await.map_err(|e| {
            DiscoveryError::ParseError(format!("Failed to parse OpenRouter response: {}", e))
        })?;

        log::info!(
            "[OpenRouter Discovery] Found {} model(s)",
            openrouter_response.data.len()
        );

        let models = openrouter_response
            .data
            .into_iter()
            .filter(|m| !Self::should_exclude(&m.id))
            .map(|m| {
                let modality_str = m.architecture.as_ref().and_then(|a| a.modality.as_deref());
                let modalities = Self::parse_modality(modality_str);

                let supports_vision =
                    modalities.contains(&ModelModality::Image) || infer_vision_support(&m.id);

                let capabilities = if supports_vision {
                    ModelCapabilities::full_featured()
                } else {
                    ModelCapabilities::text_only()
                };

                // Use context_length from API if available, otherwise infer
                let context_limit = m.context_length.or_else(|| infer_context_limit(&m.id));

                // Determine cost tier from pricing or model name
                let cost_tier = Self::infer_cost_tier_from_pricing(m.pricing.as_ref());
                let cost_tier = if cost_tier == CostTier::Medium {
                    infer_cost_tier(&m.id)
                } else {
                    cost_tier
                };

                DiscoveredModelInfo {
                    model_id: m.id.clone(),
                    display_name: m.name.or(Some(m.id.clone())),
                    provider_id: self.provider_id().to_string(),
                    cost_tier,
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
        if config.api_key.is_none() {
            return false;
        }

        let client = match create_client(5) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let url = self.discovery_url(config);
        let api_key = config.api_key.as_ref().unwrap();

        match client
            .get(&url)
            .bearer_auth(api_key)
            .header("HTTP-Referer", "https://mesoclaw.app")
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    fn discovery_url(&self, config: &ProviderConfig) -> String {
        let base = config.base_url.trim_end_matches('/');
        format!("{}/api/v1/models", base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openrouter_models_response_parsing() {
        let json = r#"{"data":[{"id":"anthropic/claude-sonnet-4","name":"Claude Sonnet 4","context_length":200000}]}"#;

        let response: OpenRouterModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "anthropic/claude-sonnet-4");
        assert_eq!(response.data[0].context_length, Some(200_000));
    }

    #[test]
    fn test_openrouter_should_exclude() {
        assert!(OpenRouterDiscovery::should_exclude(
            "openai/text-embedding-ada-002"
        ));
        assert!(!OpenRouterDiscovery::should_exclude(
            "anthropic/claude-sonnet-4"
        ));
        assert!(!OpenRouterDiscovery::should_exclude("openai/gpt-4o"));
    }

    #[test]
    fn test_openrouter_parse_modality() {
        let modalities = OpenRouterDiscovery::parse_modality(Some("text+image->text"));
        assert!(modalities.contains(&ModelModality::Text));
        assert!(modalities.contains(&ModelModality::Image));

        let modalities = OpenRouterDiscovery::parse_modality(Some("text->text"));
        assert!(modalities.contains(&ModelModality::Text));
        assert!(!modalities.contains(&ModelModality::Image));
    }

    #[test]
    fn test_openrouter_discovery_url() {
        let discovery = OpenRouterDiscovery::new();
        let config = ProviderConfig::new("openrouter", "https://openrouter.ai");
        assert_eq!(
            discovery.discovery_url(&config),
            "https://openrouter.ai/api/v1/models"
        );
    }

    #[test]
    fn test_openrouter_provider_id() {
        let discovery = OpenRouterDiscovery::new();
        assert_eq!(discovery.provider_id(), "openrouter");
    }
}
