use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Result type for AI operations
pub type Result<T> = std::result::Result<T, String>;

use super::types::{CompletionRequest, CompletionResponse, StreamChunk};

/// Type alias for streaming response
pub type StreamResponse = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

/// Trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Complete a prompt and return the full response
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Stream a completion response
    async fn stream(&self, request: CompletionRequest) -> Result<StreamResponse>;

    /// Get the context window limit for this provider
    fn context_limit(&self) -> usize;

    /// Check if this provider supports function/tool calling
    fn supports_tools(&self) -> bool;

    /// Get the provider name
    fn provider_name(&self) -> &str;

    /// Pre-establish connection to provider. Default no-op; providers may override.
    async fn warmup(&self) -> Result<()> {
        Ok(())
    }
}

/// Factory for creating LLM providers by string ID.
///
/// Delegates to [`crate::ai::providers::create_provider`].  The API key must
/// be supplied separately because the factory has no access to the OS keyring.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from a string ID and API key.
    ///
    /// Recognised IDs: `"vercel-ai-gateway"`, `"openrouter"`.
    /// Returns `Err` for unknown IDs.
    pub fn create(provider_type: &str, api_key: &str) -> Result<Box<dyn LLMProvider>> {
        use crate::ai::providers::{ProviderType, create_provider};
        let pt = ProviderType::from_id(provider_type)
            .ok_or_else(|| format!("unknown provider type: '{provider_type}'"))?;
        create_provider(pt, api_key, None, None).map(|arc| -> Box<dyn LLMProvider> {
            Box::new(crate::ai::providers::ReliableProvider::new(arc))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_factory_unknown_id() {
        let result = ProviderFactory::create("unknown-provider", "");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("unknown provider type"));
    }

    #[test]
    fn test_provider_factory_known_id() {
        assert!(ProviderFactory::create("vercel-ai-gateway", "test-key").is_ok());
    }
}
