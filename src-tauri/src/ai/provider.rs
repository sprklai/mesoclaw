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

/// Factory for creating LLM providers
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from a configuration
    pub fn create(_provider_type: &str) -> Result<Box<dyn LLMProvider>> {
        // For now, this is a placeholder
        // Will be implemented when we add specific providers
        Err("LLM provider factory not yet implemented".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_factory_placeholder() {
        let result = ProviderFactory::create("vercel");
        assert!(result.is_err());
    }
}
