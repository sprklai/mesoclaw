use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Serialize;

use crate::MesoError;
use crate::gateway::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub model_id: String,
    pub base_url: Option<String>,
}

impl ProviderInfo {
    fn from_config(config: &crate::config::AppConfig) -> Self {
        Self {
            name: config.provider_name.clone(),
            provider_type: config.provider_type.clone(),
            model_id: config.provider_model_id.clone(),
            base_url: config.provider_base_url.clone(),
        }
    }
}

/// GET /providers — list configured providers (Phase 3: single provider from config).
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> crate::Result<impl IntoResponse> {
    let provider = ProviderInfo::from_config(&state.config);
    Ok(Json(vec![provider]))
}

/// GET /providers/{id} — get a specific provider by name.
pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let provider = ProviderInfo::from_config(&state.config);
    if provider.name == id {
        Ok(Json(provider))
    } else {
        Err(MesoError::NotFound(format!("provider not found: {id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/providers", get(list_providers))
            .route("/providers/{id}", get(get_provider))
            .with_state(state)
    }

    #[tokio::test]
    async fn list_providers_returns_array() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/providers")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let providers: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0]["name"], "openai");
        assert_eq!(providers[0]["type"], "openai");
        assert_eq!(providers[0]["model_id"], "gpt-4o");
    }

    #[tokio::test]
    async fn get_provider_not_found() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/providers/nonexistent")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
