//! Model Discovery Module
//!
//! Provides traits and implementations for discovering available models
//! from various AI provider APIs.

pub mod groq;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod vercel_gateway;

// Re-export discovery implementations
pub use groq::GroqDiscovery;
pub use ollama::OllamaDiscovery;
pub use openai::OpenAIDiscovery;
pub use openrouter::OpenRouterDiscovery;
pub use vercel_gateway::VercelGatewayDiscovery;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::ai::providers::router::{CostTier, ModelCapabilities, ModelModality};
use crate::database::models::NewDiscoveredModel;

/// Error type for discovery operations
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),

    #[error("API key not configured for provider: {0}")]
    MissingApiKey(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Provider configuration for discovery
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider ID (e.g., "openai", "vercel-ai-gateway")
    pub provider_id: String,
    /// Base URL for API calls
    pub base_url: String,
    /// API key (if required)
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl ProviderConfig {
    pub fn new(provider_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            base_url: base_url.into(),
            api_key: None,
            timeout_secs: 30,
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

/// Model information returned from discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredModelInfo {
    /// Model ID as returned by the provider
    pub model_id: String,
    /// Human-readable display name
    pub display_name: Option<String>,
    /// Provider that owns this model
    pub provider_id: String,
    /// Estimated cost tier
    pub cost_tier: CostTier,
    /// Context window size if known
    pub context_limit: Option<i32>,
    /// Supported modalities
    pub modalities: Vec<ModelModality>,
    /// Model capabilities
    pub capabilities: Option<ModelCapabilities>,
    /// Whether this model supports vision/image input
    pub supports_vision: bool,
}

impl DiscoveredModelInfo {
    /// Convert to NewDiscoveredModel for database insertion
    pub fn to_new_discovered_model(&self) -> NewDiscoveredModel {
        NewDiscoveredModel::new(
            &self.provider_id,
            &self.model_id,
            self.display_name
                .clone()
                .unwrap_or_else(|| self.model_id.clone()),
            self.cost_tier,
            self.context_limit,
            self.modalities.clone(),
            self.capabilities.clone(),
        )
    }
}

/// Trait for provider-specific model discovery
#[async_trait]
pub trait ModelDiscovery: Send + Sync {
    /// Get the provider ID this discovery implementation handles
    fn provider_id(&self) -> &str;

    /// Discover available models from this provider
    async fn discover_models(
        &self,
        config: &ProviderConfig,
    ) -> Result<Vec<DiscoveredModelInfo>, DiscoveryError>;

    /// Check if this provider is available (API accessible)
    async fn is_available(&self, config: &ProviderConfig) -> bool;

    /// Get the discovery endpoint URL
    fn discovery_url(&self, config: &ProviderConfig) -> String;
}

/// Helper function to create an HTTP client with timeout
fn create_client(timeout_secs: u64) -> Result<Client, DiscoveryError> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(DiscoveryError::HttpError)
}

/// Heuristic to determine cost tier from model name
pub fn infer_cost_tier(model_id: &str) -> CostTier {
    let lower = model_id.to_lowercase();

    // Low tier indicators (check first since mini/flash override base model names)
    if lower.contains("mini")
        || lower.contains("haiku")
        || lower.contains("flash")
        || lower.contains("turbo")
        || lower.contains("lite")
        || lower.contains("small")
        || lower.contains("nano")
    {
        return CostTier::Low;
    }

    // High tier indicators
    if lower.contains("opus")
        || lower.contains("o1")
        || lower.contains("o3")
        || lower.contains("gpt-4")
        || lower.contains("gemini-ultra")
        || lower.contains("gemini-1.5-pro")
    {
        return CostTier::High;
    }

    // Default to medium
    CostTier::Medium
}

/// Heuristic to determine context limit from model name
pub fn infer_context_limit(model_id: &str) -> Option<i32> {
    let lower = model_id.to_lowercase();

    // Known context limits
    if lower.contains("claude") {
        if lower.contains("opus") || lower.contains("sonnet") {
            return Some(200_000);
        }
        return Some(200_000); // Default for Claude models
    }

    if lower.contains("gpt-4") || lower.contains("gpt4") {
        if lower.contains("turbo") || lower.contains("mini") {
            return Some(128_000);
        }
        return Some(128_000);
    }

    if lower.contains("gemini") {
        if lower.contains("2.0") || lower.contains("flash") {
            return Some(1_000_000);
        }
        if lower.contains("1.5-pro") {
            return Some(2_000_000);
        }
        return Some(1_000_000);
    }

    // Default for unknown models
    None
}

/// Heuristic to detect vision support from model name
pub fn infer_vision_support(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();

    // Models known to support vision
    lower.contains("vision")
        || lower.contains("gpt-4o")
        || lower.contains("gpt-4-turbo")
        || lower.contains("claude-3")
        || lower.contains("claude-sonnet")
        || lower.contains("claude-opus")
        || lower.contains("claude-haiku")
        || lower.contains("gemini")
        || lower.contains("llava")
        || lower.contains("cogvlm")
}

/// Default documentation URLs for each provider
pub fn default_doc_url(provider_id: &str) -> &'static str {
    match provider_id {
        "openai" => "https://developers.openai.com/api/docs/models",
        "anthropic" => "https://platform.claude.com/docs/en/about-claude/models/overview",
        "google_ai" => "https://ai.google.dev/gemini-api/docs/models",
        "groq" => "https://console.groq.com/docs/models",
        "vercel-ai-gateway" => "https://vercel.com/ai-gateway/models",
        "openrouter" => "https://openrouter.ai/models",
        "ollama" => "http://localhost:11434/api/tags",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_cost_tier() {
        assert_eq!(infer_cost_tier("claude-opus-4"), CostTier::High);
        assert_eq!(infer_cost_tier("gpt-4o"), CostTier::High);
        assert_eq!(infer_cost_tier("o1-preview"), CostTier::High);

        assert_eq!(infer_cost_tier("gpt-4o-mini"), CostTier::Low);
        assert_eq!(infer_cost_tier("claude-haiku"), CostTier::Low);
        assert_eq!(infer_cost_tier("gemini-flash"), CostTier::Low);

        assert_eq!(infer_cost_tier("claude-sonnet"), CostTier::Medium);
        assert_eq!(infer_cost_tier("unknown-model"), CostTier::Medium);
    }

    #[test]
    fn test_infer_context_limit() {
        assert_eq!(infer_context_limit("claude-opus-4"), Some(200_000));
        assert_eq!(infer_context_limit("gpt-4o"), Some(128_000));
        assert_eq!(infer_context_limit("gemini-2.0-flash"), Some(1_000_000));
    }

    #[test]
    fn test_infer_vision_support() {
        assert!(infer_vision_support("gpt-4o"));
        assert!(infer_vision_support("claude-3-opus"));
        assert!(infer_vision_support("gemini-pro-vision"));
        assert!(!infer_vision_support("gpt-3.5-turbo"));
        assert!(!infer_vision_support("text-embedding-ada-002"));
    }

    #[test]
    fn test_default_doc_url() {
        assert!(!default_doc_url("openai").is_empty());
        assert!(!default_doc_url("anthropic").is_empty());
        assert!(!default_doc_url("ollama").is_empty());
    }
}
