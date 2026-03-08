use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::ai::session::ToolCallRecord;
use crate::gateway::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct MessageWithToolCalls {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRecord>>,
}

pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<impl IntoResponse> {
    // Verify session exists first so we get a 404 for invalid sessions
    state.session_manager.get_session(&session_id).await?;
    let messages = state.session_manager.get_messages(&session_id).await?;

    let mut result = Vec::with_capacity(messages.len());
    for msg in messages {
        let tool_calls = if msg.role == "assistant" {
            let tcs = state.session_manager.get_tool_calls(&msg.id).await?;
            if tcs.is_empty() { None } else { Some(tcs) }
        } else {
            None
        };

        result.push(MessageWithToolCalls {
            id: msg.id,
            session_id: msg.session_id,
            role: msg.role,
            content: msg.content,
            created_at: msg.created_at,
            tool_calls,
        });
    }

    Ok(Json(result))
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<impl IntoResponse> {
    // Verify session exists first so we get a 404 for invalid sessions
    state.session_manager.get_session(&session_id).await?;
    let message = state
        .session_manager
        .append_message(&session_id, &req.role, &req.content)
        .await?;
    Ok((StatusCode::CREATED, Json(message)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::errors::ErrorResponse;

    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route(
                "/sessions/{session_id}/messages",
                get(get_messages).post(send_message),
            )
            .with_state(state)
    }

    // 3.3.1 — GET /sessions/{id}/messages returns 200 with array
    #[tokio::test]
    async fn get_messages_returns_array() {
        let (_dir, state) = test_state().await;
        let session = state.session_manager.create_session("Chat").await.unwrap();
        state
            .session_manager
            .append_message(&session.id, "user", "Hello")
            .await
            .unwrap();
        state
            .session_manager
            .append_message(&session.id, "assistant", "Hi there")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri(&format!("/sessions/{}/messages", session.id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let messages: Vec<crate::ai::session::Message> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "Hi there");
    }

    // 3.3.2 — POST /sessions/{id}/messages returns 201
    #[tokio::test]
    async fn post_message_returns_201() {
        let (_dir, state) = test_state().await;
        let session = state.session_manager.create_session("Chat").await.unwrap();

        let app = app(state);
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/sessions/{}/messages", session.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(
                    &serde_json::json!({"role": "user", "content": "Hello world"}),
                )
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let message: crate::ai::session::Message = serde_json::from_slice(&body).unwrap();
        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Hello world");
        assert_eq!(message.session_id, session.id);
    }

    // 3.3.3 — GET /sessions/{id}/messages for empty session returns []
    #[tokio::test]
    async fn get_messages_empty_session() {
        let (_dir, state) = test_state().await;
        let session = state
            .session_manager
            .create_session("Empty Chat")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri(&format!("/sessions/{}/messages", session.id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let messages: Vec<crate::ai::session::Message> = serde_json::from_slice(&body).unwrap();
        assert!(messages.is_empty());
    }

    // 3.3.4 — POST /sessions/{bad-id}/messages returns 404
    #[tokio::test]
    async fn post_message_invalid_session_404() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/sessions/nonexistent-id/messages")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({"role": "user", "content": "Hello"}))
                    .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.error_code, "MESO_NOT_FOUND");
    }
}
