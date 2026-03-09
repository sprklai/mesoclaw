use std::num::NonZeroUsize;
use std::sync::Mutex;

use async_trait::async_trait;
use lru::LruCache;

use crate::{MesoError, Result};

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Deterministic mock embedding provider for tests.
/// Generates embeddings based on hash of input text.
pub struct MockEmbeddingProvider {
    dim: usize,
}

impl MockEmbeddingProvider {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Deterministic: hash each char position to produce a float
        let mut vec = Vec::with_capacity(self.dim);
        for i in 0..self.dim {
            let mut hash: u64 = 5381;
            for b in text.bytes() {
                hash = hash.wrapping_mul(33).wrapping_add(b as u64);
            }
            hash = hash.wrapping_add(i as u64);
            vec.push(((hash % 10000) as f32 / 10000.0) - 0.5);
        }
        // Normalize to unit length
        let mag: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            for v in &mut vec {
                *v /= mag;
            }
        }
        Ok(vec)
    }
}

/// LRU cache wrapping an EmbeddingProvider
pub struct LruEmbeddingCache<P: EmbeddingProvider> {
    provider: P,
    cache: Mutex<LruCache<String, Vec<f32>>>,
}

impl<P: EmbeddingProvider> LruEmbeddingCache<P> {
    pub fn new(provider: P, capacity: usize) -> Self {
        Self {
            provider,
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN),
            )),
        }
    }
}

#[async_trait]
impl<P: EmbeddingProvider> EmbeddingProvider for LruEmbeddingCache<P> {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first (short Mutex hold, no await)
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|e| MesoError::Embedding(e.to_string()))?;
            if let Some(cached) = cache.get(text) {
                return Ok(cached.clone());
            }
        }
        let embedding = self.provider.embed(text).await?;
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|e| MesoError::Embedding(e.to_string()))?;
            cache.put(text.to_string(), embedding.clone());
        }
        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_zero_vector() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![0.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn cosine_similarity_mismatched_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[tokio::test]
    async fn mock_provider_deterministic() {
        let provider = MockEmbeddingProvider::new(384);
        let e1 = provider.embed("hello world").await.unwrap();
        let e2 = provider.embed("hello world").await.unwrap();
        assert_eq!(e1, e2);
    }

    #[tokio::test]
    async fn mock_provider_different_texts_differ() {
        let provider = MockEmbeddingProvider::new(384);
        let e1 = provider.embed("hello").await.unwrap();
        let e2 = provider.embed("goodbye").await.unwrap();
        assert_ne!(e1, e2);
    }

    #[tokio::test]
    async fn mock_provider_unit_normalised() {
        let provider = MockEmbeddingProvider::new(384);
        let e = provider.embed("test text").await.unwrap();
        let mag: f32 = e.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 1e-4, "magnitude was {mag}");
    }

    #[tokio::test]
    async fn lru_cache_returns_same_result() {
        let provider = MockEmbeddingProvider::new(64);
        let cache = LruEmbeddingCache::new(provider, 10);
        let e1 = cache.embed("test").await.unwrap();
        let e2 = cache.embed("test").await.unwrap();
        assert_eq!(e1, e2);
    }

    #[tokio::test]
    async fn lru_cache_different_keys() {
        let provider = MockEmbeddingProvider::new(64);
        let cache = LruEmbeddingCache::new(provider, 10);
        let e1 = cache.embed("alpha").await.unwrap();
        let e2 = cache.embed("beta").await.unwrap();
        assert_ne!(e1, e2);
    }
}
