use std::sync::Arc;

use crate::ai::provider::LLMProvider;

pub mod openai_compatible;
pub mod openrouter;
pub mod vercel_gateway;

// Public re-exports
pub use openai_compatible::{OpenAICompatibleConfig, OpenAICompatibleProvider};
pub use openrouter::OpenRouterProvider;
pub use vercel_gateway::VercelAIGatewayProvider;

// Private imports for internal use
use openrouter::OpenRouterConfig;
use vercel_gateway::VercelAIGatewayConfig;

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
    match provider_type {
        ProviderType::VercelAIGateway => {
            let mut config = VercelAIGatewayConfig::new(api_key);
            if let Some(url) = base_url {
                config = config.with_base_url(url);
            }
            if let Some(model) = default_model {
                config = config.with_default_model(model);
            }
            Ok(Arc::new(VercelAIGatewayProvider::new(config)?))
        }
        ProviderType::OpenRouter => {
            let mut config = OpenRouterConfig::new(api_key);
            if let Some(url) = base_url {
                config = config.with_base_url(url);
            }
            if let Some(model) = default_model {
                config = config.with_default_model(model);
            }
            Ok(Arc::new(OpenRouterProvider::new(config)?))
        }
    }
}

/*
// Database-specific provider creation - commented out for boilerplate
// Re-implement if you need database-backed provider configuration

/// Create a provider from database configuration
pub async fn create_provider_from_db(
    pool: &crate::database::DbPool,
    provider_id: &str,
    api_key: &str,
) -> Result<Arc<dyn LLMProvider>, String> {
    // Implementation removed - database-specific
    Err("Database-backed provider creation not implemented in boilerplate".to_string())
}
*/

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
}

