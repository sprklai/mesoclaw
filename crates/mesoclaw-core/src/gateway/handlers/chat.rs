use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::ai::resolve_agent;
use crate::gateway::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub session_id: Option<String>,
}

pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<impl IntoResponse> {
    let agent = resolve_agent(req.model.as_deref(), &state, None).await?;

    // If session_id provided, store the user message
    if let Some(ref sid) = req.session_id {
        let _ = state
            .session_manager
            .append_message(sid, "user", &req.prompt)
            .await;
    }

    let response = agent.prompt(&req.prompt).await?;

    // If session_id provided, store the assistant response
    if let Some(ref sid) = req.session_id {
        let _ = state
            .session_manager
            .append_message(sid, "assistant", &response)
            .await;
    }

    Ok(Json(ChatResponse {
        response,
        session_id: req.session_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use serde_json::json;
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new().route("/chat", post(chat)).with_state(state)
    }

    // 3.4.2 — chat empty body returns 422 (missing required field)
    #[tokio::test]
    async fn chat_empty_body_returns_422() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("POST")
            .uri("/chat")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    // 3.4.1 — chat post with no agent returns 502
    #[tokio::test]
    async fn chat_no_agent_returns_502() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("POST")
            .uri("/chat")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({"prompt": "hello"})).unwrap(),
            ))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }
}
