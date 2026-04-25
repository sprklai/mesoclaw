// Module is already feature-gated in memory/mod.rs

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, OnceCell};

use crate::{Result, ZeniiError};

use super::embeddings::EmbeddingProvider;

/// Local embedding provider using fastembed (ONNX Runtime).
/// Feature-gated behind `local-embeddings`.
pub struct FastEmbedProvider {
    model: OnceCell<Arc<Mutex<fastembed::TextEmbedding>>>,
    model_name: fastembed::EmbeddingModel,
    cache_dir: Option<std::path::PathBuf>,
}

impl FastEmbedProvider {
    pub fn new(model_name: &str, cache_dir: Option<std::path::PathBuf>) -> Result<Self> {
        let model = match model_name {
            "bge-small-en-v1.5" | "BGESmallENV15" => fastembed::EmbeddingModel::BGESmallENV15,
            "bge-small-en-v1.5-q" | "BGESmallENV15Q" => fastembed::EmbeddingModel::BGESmallENV15Q,
            "all-MiniLM-L6-v2" | "AllMiniLML6V2" => fastembed::EmbeddingModel::AllMiniLML6V2,
            "all-MiniLM-L6-v2-q" | "AllMiniLML6V2Q" => fastembed::EmbeddingModel::AllMiniLML6V2Q,
            _ => {
                return Err(ZeniiError::Embedding(format!(
                    "unsupported model: {model_name}"
                )));
            }
        };

        Ok(Self {
            model: OnceCell::new(),
            model_name: model,
            cache_dir,
        })
    }

    async fn get_model(&self) -> Result<Arc<Mutex<fastembed::TextEmbedding>>> {
        self.model
            .get_or_try_init(|| async {
                let model_name = self.model_name.clone();
                let cache_dir = self.cache_dir.clone();

                tracing::info!("Initializing fastembed model (may download on first use)...");

                let model = tokio::task::spawn_blocking(move || {
                    let mut opts =
                        fastembed::InitOptions::new(model_name).with_show_download_progress(true);
                    if let Some(dir) = cache_dir {
                        opts = opts.with_cache_dir(dir);
                    }
                    fastembed::TextEmbedding::try_new(opts)
                        .map_err(|e| ZeniiError::Embedding(format!("fastembed init failed: {e}")))
                })
                .await
                .map_err(|e| ZeniiError::Embedding(format!("spawn_blocking failed: {e}")))??;

                tracing::info!("Fastembed model ready");
                Ok(Arc::new(Mutex::new(model)))
            })
            .await
            .cloned()
    }
}

#[async_trait]
impl EmbeddingProvider for FastEmbedProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if text.trim().is_empty() {
            return Err(ZeniiError::Embedding("text cannot be empty".into()));
        }

        let model = self.get_model().await?;
        let text = text.to_string();

        tokio::task::spawn_blocking(move || {
            let mut guard = model.blocking_lock();
            let embeddings = guard
                .embed(vec![text], None)
                .map_err(|e| ZeniiError::Embedding(format!("embed failed: {e}")))?;
            embeddings
                .into_iter()
                .next()
                .ok_or_else(|| ZeniiError::Embedding("empty embedding result".into()))
        })
        .await
        .map_err(|e| ZeniiError::Embedding(format!("spawn_blocking failed: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 18.5 — Provider initializes with default model
    #[test]
    fn fastembed_init_default() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None);
        assert!(provider.is_ok());
    }

    // 18.6 — Embed returns 384-dim vector
    #[tokio::test]
    #[ignore = "requires ONNX model download, run locally with --ignored"]
    async fn fastembed_embed_dimensions() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        let result = provider.embed("hello world").await.unwrap();
        assert_eq!(result.len(), 384);
    }

    // 18.7 — Same text produces same embedding (deterministic)
    #[tokio::test]
    #[ignore = "requires ONNX model download, run locally with --ignored"]
    async fn fastembed_deterministic() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        let e1 = provider.embed("test text").await.unwrap();
        let e2 = provider.embed("test text").await.unwrap();
        assert_eq!(e1, e2);
    }

    // 18.8 — Different texts produce different embeddings
    #[tokio::test]
    #[ignore = "requires ONNX model download, run locally with --ignored"]
    async fn fastembed_different_texts() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        let e1 = provider.embed("hello world").await.unwrap();
        let e2 = provider.embed("goodbye moon").await.unwrap();
        assert_ne!(e1, e2);
    }

    // 18.9 — Embedding is unit-normalized
    #[tokio::test]
    #[ignore = "requires ONNX model download, run locally with --ignored"]
    async fn fastembed_normalized() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        let embedding = provider.embed("test normalization").await.unwrap();
        let mag: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (mag - 1.0).abs() < 0.01,
            "embedding should be unit-normalized, got magnitude {mag}"
        );
    }
}
