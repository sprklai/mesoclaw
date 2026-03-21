use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::ai::session::{DelegationRecord, ToolCallRecord};
use crate::event_bus::AppEvent;
use crate::gateway::state::AppState;

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct SendMessageRequest {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct MessageWithToolCalls {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation: Option<DelegationRecord>,
}

#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/sessions/{id}/messages", tag = "Messages",
    params(("id" = String, Path, description = "Session ID")),
    responses(
        (status = 200, description = "List of messages with tool calls", body = Vec<MessageWithToolCalls>),
        (status = 404, description = "Session not found", body = Object),
    )
))]
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

        let delegation = if msg.role == "assistant" {
            state.session_manager.get_delegation(&msg.id).await?
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
            delegation,
        });
    }

    Ok(Json(result))
}

#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/sessions/{id}/messages", tag = "Messages",
    params(("id" = String, Path, description = "Session ID")),
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Message created", body = Object),
        (status = 404, description = "Session not found", body = Object),
    )
))]
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
    let _ = state.event_bus.publish(AppEvent::MessageAdded {
        session_id: session_id.clone(),
        message_id: message.id.clone(),
        role: req.role,
    });
    Ok((StatusCode::CREATED, Json(message)))
}

#[cfg_attr(feature = "api-docs", utoipa::path(
    delete, path = "/sessions/{id}/messages/{message_id}/and-after", tag = "Messages",
    params(
        ("id" = String, Path, description = "Session ID"),
        ("message_id" = String, Path, description = "Message ID to delete from"),
    ),
    responses(
        (status = 204, description = "Messages deleted"),
        (status = 404, description = "Session or message not found", body = Object),
    )
))]
pub async fn delete_messages_from(
    State(state): State<Arc<AppState>>,
    Path((session_id, message_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    state.session_manager.get_session(&session_id).await?;
    state
        .session_manager
        .delete_messages_from(&session_id, &message_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::errors::ErrorResponse;

    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::{delete, get};
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
            .route(
                "/sessions/{session_id}/messages/{message_id}/and-after",
                delete(delete_messages_from),
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
        assert_eq!(error.error_code, "ZENII_NOT_FOUND");
    }

    // EDIT.H1 — DELETE /sessions/{id}/messages/{mid}/and-after returns 204
    #[tokio::test]
    async fn delete_messages_from_returns_204() {
        let (_dir, state) = test_state().await;
        let session = state.session_manager.create_session("Chat").await.unwrap();
        let m1 = state
            .session_manager
            .append_message(&session.id, "user", "First")
            .await
            .unwrap();
        let m2 = state
            .session_manager
            .append_message(&session.id, "assistant", "Second")
            .await
            .unwrap();
        let _m3 = state
            .session_manager
            .append_message(&session.id, "user", "Third")
            .await
            .unwrap();

        let app = app(state.clone());
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!(
                "/sessions/{}/messages/{}/and-after",
                session.id, m2.id
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify only m1 remains
        let remaining = state
            .session_manager
            .get_messages(&session.id)
            .await
            .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, m1.id);
    }

    // EDIT.H2 — DELETE with invalid session returns 404
    #[tokio::test]
    async fn delete_messages_from_invalid_session_404() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/sessions/bad-session/messages/bad-msg/and-after")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // EDIT.H3 — DELETE with invalid message returns 404
    #[tokio::test]
    async fn delete_messages_from_invalid_message_404() {
        let (_dir, state) = test_state().await;
        let session = state.session_manager.create_session("Chat").await.unwrap();
        state
            .session_manager
            .append_message(&session.id, "user", "Hello")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!(
                "/sessions/{}/messages/nonexistent-msg/and-after",
                session.id
            ))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
