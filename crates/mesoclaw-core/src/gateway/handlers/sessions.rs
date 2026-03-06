use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::Result;
use crate::gateway::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub title: String,
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<impl IntoResponse> {
    let session = state.session_manager.create_session(&req.title).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn list_sessions(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse> {
    let sessions = state.session_manager.list_sessions().await?;
    Ok(Json(sessions))
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let session = state.session_manager.get_session(&id).await?;
    Ok(Json(session))
}

pub async fn update_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSessionRequest>,
) -> Result<impl IntoResponse> {
    let session = state
        .session_manager
        .update_session(&id, &req.title)
        .await?;
    Ok(Json(session))
}

pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    state.session_manager.delete_session(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::errors::ErrorResponse;

    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::{get, post};
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/sessions", post(create_session).get(list_sessions))
            .route(
                "/sessions/{id}",
                get(get_session).put(update_session).delete(delete_session),
            )
            .with_state(state)
    }

    // 3.2.1 — POST /sessions returns 201 with session JSON
    #[tokio::test]
    async fn create_session_returns_201() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({"title": "Test Session"})).unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let session: crate::ai::session::Session = serde_json::from_slice(&body).unwrap();
        assert_eq!(session.title, "Test Session");
        assert!(!session.id.is_empty());
    }

    // 3.2.2 — GET /sessions returns 200 with array
    #[tokio::test]
    async fn list_sessions_returns_array() {
        let (_dir, state) = test_state().await;

        // Create two sessions first
        state.session_manager.create_session("First").await.unwrap();
        state
            .session_manager
            .create_session("Second")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri("/sessions")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<crate::ai::session::SessionSummary> =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    // 3.2.3 — GET /sessions/{id} returns 200
    #[tokio::test]
    async fn get_session_returns_200() {
        let (_dir, state) = test_state().await;
        let created = state
            .session_manager
            .create_session("My Session")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri(&format!("/sessions/{}", created.id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let session: crate::ai::session::Session = serde_json::from_slice(&body).unwrap();
        assert_eq!(session.id, created.id);
        assert_eq!(session.title, "My Session");
    }

    // 3.2.4 — GET /sessions/{bad-id} returns 404
    #[tokio::test]
    async fn get_session_not_found_returns_404() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/sessions/nonexistent-id")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.error_code, "MESO_NOT_FOUND");
    }

    // 3.2.5 — PUT /sessions/{id} returns 200
    #[tokio::test]
    async fn update_session_returns_200() {
        let (_dir, state) = test_state().await;
        let created = state
            .session_manager
            .create_session("Old Title")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .method("PUT")
            .uri(&format!("/sessions/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({"title": "New Title"})).unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let session: crate::ai::session::Session = serde_json::from_slice(&body).unwrap();
        assert_eq!(session.title, "New Title");
        assert_eq!(session.id, created.id);
    }

    // 3.2.6 — DELETE /sessions/{id} returns 204
    #[tokio::test]
    async fn delete_session_returns_204() {
        let (_dir, state) = test_state().await;
        let created = state
            .session_manager
            .create_session("To Delete")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/sessions/{}", created.id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // 3.2.7 — DELETE /sessions/{bad-id} returns 404
    #[tokio::test]
    async fn delete_session_not_found_returns_404() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/sessions/nonexistent-id")
            .body(Body::empty())
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
