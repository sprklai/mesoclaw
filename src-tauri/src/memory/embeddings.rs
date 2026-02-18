//! Embedding generation and caching for the memory subsystem.
//!
//! This module provides:
//! - [`EmbeddingProvider`] — async trait for computing text embeddings
//! - [`MockEmbeddingProvider`] — deterministic hash-based embeddings for tests
//! - [`LruEmbeddingCache`] — LRU-cached wrapper over any provider
//! - [`cosine_similarity`] — utility function for vector similarity

use std::sync::Mutex;

use async_trait::async_trait;
use lru::LruCache;

// ─── Dimension constant ───────────────────────────────────────────────────────

/// Embedding dimension used throughout the memory subsystem.
/// Matches `text-embedding-3-small` (OpenAI) and common Ollama models.
pub const EMBEDDING_DIM: usize = 384;

// ─── EmbeddingProvider ───────────────────────────────────────────────────────

/// Async trait for computing fixed-dimension text embeddings.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Return a normalised embedding vector for `text`.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String>;
}

// ─── cosine_similarity ───────────────────────────────────────────────────────

/// Compute cosine similarity between two vectors.
///
/// Returns a value in `[0, 1]` (vectors are expected to be unit-normalised).
/// Returns `0.0` if either vector is all-zeros.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
    }
}

// ─── MockEmbeddingProvider ───────────────────────────────────────────────────

/// Deterministic, hash-based embedding provider for tests and offline use.
///
/// Produces a unit-normalised vector whose values are derived from a simple
/// character-hash of the input text.  Different texts produce different (but
/// consistent) vectors; identical texts always produce the same vector.
#[derive(Debug, Default)]
pub struct MockEmbeddingProvider;

impl MockEmbeddingProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // Hash-based deterministic embedding.
        let mut raw = vec![0.0f32; EMBEDDING_DIM];
        for (i, ch) in text.chars().enumerate() {
            let idx = (i + ch as usize) % EMBEDDING_DIM;
            raw[idx] += 1.0;
        }
        // Normalise to unit vector.
        let mag: f32 = raw.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            for v in &mut raw {
                *v /= mag;
            }
        }
        Ok(raw)
    }
}

// ─── LruEmbeddingCache ───────────────────────────────────────────────────────

/// LRU-cached wrapper over any [`EmbeddingProvider`].
///
/// Avoids redundant API calls for texts that have already been embedded.
/// The cache key is the exact input text; capacity defaults to 10 000 entries.
pub struct LruEmbeddingCache {
    inner: Box<dyn EmbeddingProvider>,
    cache: Mutex<LruCache<String, Vec<f32>>>,
}

impl LruEmbeddingCache {
    /// Create a cache wrapping `provider` with the given `capacity`.
    pub fn new(provider: Box<dyn EmbeddingProvider>, capacity: usize) -> Self {
        let cap = std::num::NonZeroUsize::new(capacity).unwrap_or(
            // Safety: 10_000 is non-zero.
            std::num::NonZeroUsize::MIN.saturating_add(9_999),
        );
        Self {
            inner: provider,
            cache: Mutex::new(LruCache::new(cap)),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for LruEmbeddingCache {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // Check cache first (no await inside lock).
        {
            let mut guard = self.cache.lock().map_err(|e| e.to_string())?;
            if let Some(hit) = guard.get(text) {
                return Ok(hit.clone());
            }
        }
        // Cache miss — delegate to inner provider.
        let embedding = self.inner.embed(text).await?;
        {
            let mut guard = self.cache.lock().map_err(|e| e.to_string())?;
            guard.put(text.to_owned(), embedding.clone());
        }
        Ok(embedding)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical() {
        let v = vec![0.6f32, 0.8, 0.0];
        let score = cosine_similarity(&v, &v);
        assert!((score - 1.0).abs() < 1e-5, "identical vectors → 1.0, got {score}");
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        let score = cosine_similarity(&a, &b);
        assert!((score - 0.0).abs() < 1e-5, "orthogonal vectors → 0.0, got {score}");
    }

    #[test]
    fn cosine_similarity_zero_vector_no_panic() {
        let a = vec![0.0f32; 4];
        let b = vec![1.0f32, 0.0, 0.0, 0.0];
        let score = cosine_similarity(&a, &b);
        assert_eq!(score, 0.0, "zero vector → 0.0");
    }

    #[test]
    fn cosine_similarity_mismatched_lengths() {
        let a = vec![1.0f32, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        let score = cosine_similarity(&a, &b);
        assert_eq!(score, 0.0, "mismatched lengths → 0.0");
    }

    #[tokio::test]
    async fn mock_provider_deterministic() {
        let provider = MockEmbeddingProvider::new();
        let e1 = provider.embed("hello world").await.unwrap();
        let e2 = provider.embed("hello world").await.unwrap();
        assert_eq!(e1, e2, "same text → same embedding");
    }

    #[tokio::test]
    async fn mock_provider_different_texts_differ() {
        let provider = MockEmbeddingProvider::new();
        let e1 = provider.embed("hello world").await.unwrap();
        let e2 = provider.embed("goodbye world").await.unwrap();
        assert_ne!(e1, e2, "different texts → different embeddings");
    }

    #[tokio::test]
    async fn mock_provider_unit_normalised() {
        let provider = MockEmbeddingProvider::new();
        let e = provider.embed("normalise me").await.unwrap();
        let mag: f32 = e.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 1e-5, "embedding should be unit-length, got {mag}");
    }

    #[tokio::test]
    async fn lru_cache_returns_same_result() {
        let cache = LruEmbeddingCache::new(Box::new(MockEmbeddingProvider::new()), 10);
        let first = cache.embed("test text").await.unwrap();
        let second = cache.embed("test text").await.unwrap();
        assert_eq!(first, second, "cached value should match original");
    }

    #[tokio::test]
    async fn lru_cache_different_keys() {
        let cache = LruEmbeddingCache::new(Box::new(MockEmbeddingProvider::new()), 10);
        let a = cache.embed("text a").await.unwrap();
        let b = cache.embed("text b").await.unwrap();
        assert_ne!(a, b, "different texts should produce different embeddings");
    }
}
