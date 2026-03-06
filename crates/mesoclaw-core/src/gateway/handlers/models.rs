use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;

use crate::gateway::state::AppState;

/// GET /models -- returns list of available models from config.
pub async fn list_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let config = &state.config;
    Json(json!([
        {
            "id": config.provider_model_id,
            "provider": config.provider_name,
        }
    ]))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tempfile::TempDir;
    use tower::ServiceExt;

    use super::*;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    #[tokio::test]
    async fn models_list_returns_array() {
        let (_dir, state) = test_state().await;
        let app = Router::new()
            .route("/models", get(list_models))
            .with_state(state);

        let req = Request::builder()
            .uri("/models")
            .body(Body::empty())
            .expect("build request");

        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");

        let arr = json.as_array().expect("response should be an array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "gpt-4o");
        assert_eq!(arr[0]["provider"], "openai");
    }
}
