use std::sync::Arc;

use crate::ai::provider::LLMProvider;

pub mod openai_compatible;

// Public re-exports
pub use openai_compatible::{OpenAICompatibleConfig, OpenAICompatibleProvider};

/// Provider type enum for known providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    VercelAIGateway,
    OpenRouter,
}

impl ProviderType {
    /// Parse provider type from string ID
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "vercel-ai-gateway" => Some(ProviderType::VercelAIGateway),
            "openrouter" => Some(ProviderType::OpenRouter),
            _ => None,
        }
    }

    /// Get the provider ID string
    pub fn as_id(&self) -> &'static str {
        match self {
            ProviderType::VercelAIGateway => "vercel-ai-gateway",
            ProviderType::OpenRouter => "openrouter",
        }
    }
}

/// Create a provider from type and API key
pub fn create_provider(
    provider_type: ProviderType,
    api_key: &str,
    base_url: Option<&str>,
    default_model: Option<&str>,
) -> Result<Arc<dyn LLMProvider>, String> {
    let (mut config, name) = match provider_type {
        ProviderType::VercelAIGateway => (
            OpenAICompatibleConfig::vercel_gateway(api_key),
            "vercel-ai-gateway",
        ),
        ProviderType::OpenRouter => (
            OpenAICompatibleConfig::openrouter(api_key),
            "openrouter",
        ),
    };

    if let Some(url) = base_url {
        config.base_url = url.to_string();
    }
    if let Some(model) = default_model {
        config.default_model = model.to_string();
    }

    Ok(Arc::new(OpenAICompatibleProvider::new(config, name)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_from_id() {
        assert_eq!(
            ProviderType::from_id("vercel-ai-gateway"),
            Some(ProviderType::VercelAIGateway)
        );
        assert_eq!(
            ProviderType::from_id("openrouter"),
            Some(ProviderType::OpenRouter)
        );
        assert_eq!(ProviderType::from_id("unknown"), None);
    }

    #[test]
    fn test_provider_type_as_id() {
        assert_eq!(ProviderType::VercelAIGateway.as_id(), "vercel-ai-gateway");
        assert_eq!(ProviderType::OpenRouter.as_id(), "openrouter");
    }

    #[test]
    fn test_create_vercel_provider() {
        let result = create_provider(
            ProviderType::VercelAIGateway,
            "test-key",
            Some("https://custom.api.com"),
            Some("google/gemini-3-flash"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_openrouter_provider() {
        let result = create_provider(
            ProviderType::OpenRouter,
            "test-key",
            Some("https://custom.api.com"),
            Some("anthropic/claude-3.5-sonnet"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_openrouter_has_required_headers() {
        let config = OpenAICompatibleConfig::openrouter("test-key");
        assert!(config.extra_headers.contains_key("HTTP-Referer"));
        assert!(config.extra_headers.contains_key("X-Title"));
    }
}
