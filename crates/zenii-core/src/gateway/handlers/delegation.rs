use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::gateway::state::AppState;
use crate::{Result, ZeniiError};

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct ActiveAgentsResponse {
    pub agents: Vec<String>,
}

pub async fn list_active_agents(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agents = state.coordinator.active_agents();
    Json(ActiveAgentsResponse { agents })
}

pub async fn cancel_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    if state.coordinator.cancel(&id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ZeniiError::NotFound(format!(
            "delegation '{id}' not found"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::handlers::chat::ChatRequest;

    // 7.21
    #[test]
    fn chat_request_with_delegation_field() {
        let json = r#"{"prompt":"hello","delegation":true}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.delegation, Some(true));
    }

    // 7.22
    #[test]
    fn chat_request_without_delegation() {
        let json = r#"{"prompt":"hello"}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert!(req.delegation.is_none());
    }

    // 7.23
    #[tokio::test]
    async fn list_active_agents_empty() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/agents/active")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: ActiveAgentsResponse = serde_json::from_slice(&body).unwrap();
        assert!(parsed.agents.is_empty());
    }

    // 7.24
    #[tokio::test]
    async fn cancel_nonexistent_agent() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/agents/nonexistent-id/cancel")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
