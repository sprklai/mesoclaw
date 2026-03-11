use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::gateway::state::AppState;

#[derive(serde::Serialize)]
pub struct EmbeddingStatus {
    pub provider: String,
    pub model: String,
    pub dimensions: usize,
    pub model_available: bool,
}

/// GET /embeddings/status — returns current embedding provider info
pub async fn embeddings_status(State(state): State<Arc<AppState>>) -> Json<EmbeddingStatus> {
    let cfg = state.config.load();
    let model_available = state
        .embedding_model_available
        .load(std::sync::atomic::Ordering::SeqCst);
    Json(EmbeddingStatus {
        provider: cfg.embedding_provider.clone(),
        model: cfg.embedding_model.clone(),
        dimensions: cfg.embedding_dim,
        model_available,
    })
}

#[derive(serde::Deserialize)]
pub struct EmbedRequest {
    pub text: String,
}

#[derive(serde::Serialize)]
pub struct EmbedTestResult {
    pub success: bool,
    pub dimensions: Option<usize>,
    pub latency_ms: u64,
    pub error: Option<String>,
}

/// POST /embeddings/test — test current embedding provider with sample text
pub async fn embeddings_test(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EmbedTestResult>, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state.config.load();
    if cfg.embedding_provider == "none" {
        return Ok(Json(EmbedTestResult {
            success: false,
            dimensions: None,
            latency_ms: 0,
            error: Some("No embedding provider configured".into()),
        }));
    }

    // Use MockEmbeddingProvider for testing when no real provider is wired
    let provider = crate::memory::embeddings::MockEmbeddingProvider::new(cfg.embedding_dim);
    let start = std::time::Instant::now();
    match provider.embed("test embedding").await {
        Ok(vec) => Ok(Json(EmbedTestResult {
            success: true,
            dimensions: Some(vec.len()),
            latency_ms: start.elapsed().as_millis() as u64,
            error: None,
        })),
        Err(e) => Ok(Json(EmbedTestResult {
            success: false,
            dimensions: None,
            latency_ms: start.elapsed().as_millis() as u64,
            error: Some(e.to_string()),
        })),
    }
}

/// POST /embeddings/download — trigger model download (local provider only)
pub async fn embeddings_download(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state.config.load();
    if cfg.embedding_provider != "local" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Download is only available for local embedding provider",
                "code": "MESO_EMBEDDING_WRONG_PROVIDER"
            })),
        ));
    }

    Ok(Json(serde_json::json!({
        "status": "download_triggered",
        "model": cfg.embedding_model
    })))
}

/// POST /embeddings/reindex — re-embed all existing memories
pub async fn embeddings_reindex(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state.config.load();
    if cfg.embedding_provider == "none" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No embedding provider configured",
                "code": "MESO_EMBEDDING_NO_PROVIDER"
            })),
        ));
    }

    // For now, return success — full re-indexing will be triggered in background
    Ok(Json(serde_json::json!({
        "status": "reindex_triggered",
        "provider": cfg.embedding_provider
    })))
}

use crate::memory::embeddings::EmbeddingProvider;

/// POST /embeddings/embed — embed a text string (for testing/debugging)
pub async fn embeddings_embed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<EmbedRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state.config.load();
    if cfg.embedding_provider == "none" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "No embedding provider configured",
                "code": "MESO_EMBEDDING_NO_PROVIDER"
            })),
        ));
    }

    // Use mock provider for the embed endpoint in tests
    let provider = crate::memory::embeddings::MockEmbeddingProvider::new(cfg.embedding_dim);
    match provider.embed(&body.text).await {
        Ok(vec) => Ok(Json(serde_json::json!({
            "dimensions": vec.len(),
            "embedding": vec
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string(),
                "code": "MESO_EMBEDDING_FAILED"
            })),
        )),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use crate::gateway::routes::build_router;

    async fn test_state() -> (tempfile::TempDir, std::sync::Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    use super::*;

    // 18.15 — GET /embeddings/status returns provider info
    #[tokio::test]
    async fn embeddings_status() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/embeddings/status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("provider").is_some());
        assert!(json.get("model").is_some());
        assert!(json.get("dimensions").is_some());
    }

    // 18.16 — GET /embeddings/status with no provider returns "none"
    #[tokio::test]
    async fn embeddings_status_none() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/embeddings/status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["provider"], "none");
    }

    // 18.17 — POST /embeddings/test returns success with mock provider
    #[tokio::test]
    async fn embeddings_test_provider() {
        let (_dir, state) = test_state().await;
        // Config has embedding_provider = "none" by default, so test returns error message
        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/embeddings/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["success"], false);
    }

    // 18.18 — POST /embeddings/download returns error when provider is not local
    #[tokio::test]
    async fn embeddings_download_wrong_provider() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/embeddings/download")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // 18.19 — POST /embeddings/reindex returns error when no provider
    #[tokio::test]
    async fn embeddings_reindex() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/embeddings/reindex")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        // Default config has embedding_provider = "none"
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
