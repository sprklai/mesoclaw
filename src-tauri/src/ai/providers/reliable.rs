//! ReliableProvider — retry + fallback wrapper around any LLMProvider.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::ai::provider::{LLMProvider, Result, StreamResponse};
use crate::ai::types::{CompletionRequest, CompletionResponse};

/// Wraps a primary provider with retry-with-exponential-backoff and an
/// optional ordered fallback chain.
pub struct ReliableProvider {
    primary: Arc<dyn LLMProvider>,
    fallbacks: Vec<Arc<dyn LLMProvider>>,
    max_retries: u32,
    base_delay: Duration,
}

impl ReliableProvider {
    /// Create a new `ReliableProvider` with the given primary provider.
    pub fn new(primary: Arc<dyn LLMProvider>) -> Self {
        Self {
            primary,
            fallbacks: vec![],
            max_retries: 3,
            base_delay: Duration::from_millis(500),
        }
    }

    /// Set the retry configuration.
    pub fn with_retries(mut self, max_retries: u32, base_delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.base_delay = base_delay;
        self
    }

    /// Add a fallback provider tried after all retries on the primary fail.
    pub fn with_fallback(mut self, provider: Arc<dyn LLMProvider>) -> Self {
        self.fallbacks.push(provider);
        self
    }
}

#[async_trait]
impl LLMProvider for ReliableProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Try primary with retries
        let mut last_err = String::new();
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.base_delay * 2u32.saturating_pow(attempt - 1);
                tokio::time::sleep(delay).await;
            }
            match self.primary.complete(request.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) => last_err = e,
            }
        }

        // Retries exhausted — try fallbacks
        for fallback in &self.fallbacks {
            match fallback.complete(request.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) => last_err = e,
            }
        }

        Err(format!("All providers failed. Last error: {last_err}"))
    }

    async fn stream(&self, request: CompletionRequest) -> Result<StreamResponse> {
        // Try primary with retries
        let mut last_err = String::new();
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = self.base_delay * 2u32.saturating_pow(attempt - 1);
                tokio::time::sleep(delay).await;
            }
            match self.primary.stream(request.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => last_err = e,
            }
        }

        // Retries exhausted — try fallbacks
        for fallback in &self.fallbacks {
            match fallback.stream(request.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(e) => last_err = e,
            }
        }

        Err(format!("All providers failed. Last error: {last_err}"))
    }

    /// Returns the minimum context limit across the primary provider and all
    /// fallbacks so that callers never submit requests that would exceed a
    /// fallback provider's capacity.
    fn context_limit(&self) -> usize {
        self.fallbacks
            .iter()
            .map(|f| f.context_limit())
            .fold(self.primary.context_limit(), |min, limit| min.min(limit))
    }

    /// Returns `true` only when every provider in the chain (primary and all
    /// fallbacks) supports tool use, ensuring that tool-based requests remain
    /// valid after a fallback switch.
    fn supports_tools(&self) -> bool {
        self.primary.supports_tools() && self.fallbacks.iter().all(|f| f.supports_tools())
    }

    fn provider_name(&self) -> &str {
        self.primary.provider_name()
    }

    async fn warmup(&self) -> Result<()> {
        // Warm up primary; log failures but don't propagate (provider may come online later)
        if let Err(e) = self.primary.warmup().await {
            log::warn!(
                "ReliableProvider warmup failed for {}: {e}",
                self.primary.provider_name()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::provider::LLMProvider;
    use crate::ai::types::{CompletionRequest, CompletionResponse, StreamChunk};
    use async_trait::async_trait;
    use futures::stream;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// Provider that always fails.
    struct AlwaysFailProvider {
        name: &'static str,
        call_count: Arc<AtomicU32>,
    }

    impl AlwaysFailProvider {
        fn new(name: &'static str) -> (Self, Arc<AtomicU32>) {
            let count = Arc::new(AtomicU32::new(0));
            (
                Self {
                    name,
                    call_count: count.clone(),
                },
                count,
            )
        }
    }

    #[async_trait]
    impl LLMProvider for AlwaysFailProvider {
        async fn complete(&self, _: CompletionRequest) -> Result<CompletionResponse> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Err(format!("{} failed", self.name))
        }
        async fn stream(&self, _: CompletionRequest) -> Result<StreamResponse> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Err(format!("{} stream failed", self.name))
        }
        fn context_limit(&self) -> usize {
            4096
        }
        fn supports_tools(&self) -> bool {
            false
        }
        fn provider_name(&self) -> &str {
            self.name
        }
    }

    /// Provider that always succeeds.
    struct AlwaysOkProvider;

    #[async_trait]
    impl LLMProvider for AlwaysOkProvider {
        async fn complete(&self, _: CompletionRequest) -> Result<CompletionResponse> {
            Ok(CompletionResponse {
                content: "ok".to_string(),
                model: "test".to_string(),
                usage: None,
                finish_reason: None,
            })
        }
        async fn stream(&self, _: CompletionRequest) -> Result<StreamResponse> {
            Ok(Box::pin(stream::empty::<Result<StreamChunk>>()))
        }
        fn context_limit(&self) -> usize {
            4096
        }
        fn supports_tools(&self) -> bool {
            false
        }
        fn provider_name(&self) -> &str {
            "always-ok"
        }
    }

    fn dummy_request() -> CompletionRequest {
        CompletionRequest::new("test-model", vec![])
    }

    #[tokio::test]
    async fn test_retries_on_primary_failure() {
        let (fail_provider, count) = AlwaysFailProvider::new("primary");
        let reliable = ReliableProvider::new(Arc::new(fail_provider))
            .with_retries(2, Duration::from_millis(1));

        let result = reliable.complete(dummy_request()).await;
        assert!(result.is_err());
        // 1 initial + 2 retries = 3 calls
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_fallback_chain_activated() {
        let (fail_primary, _) = AlwaysFailProvider::new("primary");
        let reliable = ReliableProvider::new(Arc::new(fail_primary))
            .with_retries(0, Duration::from_millis(1))
            .with_fallback(Arc::new(AlwaysOkProvider));

        let result = reliable.complete(dummy_request()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "ok");
    }

    #[tokio::test]
    async fn test_max_retries_respected() {
        let (fail_provider, count) = AlwaysFailProvider::new("primary");
        let reliable = ReliableProvider::new(Arc::new(fail_provider))
            .with_retries(1, Duration::from_millis(1));

        let _ = reliable.complete(dummy_request()).await;
        assert_eq!(count.load(Ordering::SeqCst), 2); // 1 initial + 1 retry
    }

    #[tokio::test]
    async fn test_warmup_failure_does_not_propagate() {
        let (fail_provider, _) = AlwaysFailProvider::new("warmup-fail");
        let reliable = ReliableProvider::new(Arc::new(fail_provider))
            .with_retries(0, Duration::from_millis(1));

        // warmup() should succeed even though underlying provider would fail complete/stream
        assert!(reliable.warmup().await.is_ok());
    }
}
