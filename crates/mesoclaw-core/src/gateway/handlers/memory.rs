use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::MesoError;
use crate::gateway::state::AppState;
use crate::memory::traits::MemoryCategory;

#[derive(Deserialize)]
pub struct StoreMemoryRequest {
    pub key: String,
    pub content: String,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMemoryRequest {
    pub content: String,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct RecallQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

fn parse_category(cat: Option<&str>) -> MemoryCategory {
    match cat {
        Some(s) => MemoryCategory::from(s),
        None => MemoryCategory::Core,
    }
}

/// POST /memory — store a new memory entry.
pub async fn create_memory(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StoreMemoryRequest>,
) -> crate::Result<impl IntoResponse> {
    let category = parse_category(body.category.as_deref());
    state
        .memory
        .store(&body.key, &body.content, category)
        .await?;
    Ok(StatusCode::CREATED)
}

/// GET /memory?q=search_term&limit=N — recall memories matching a query.
pub async fn recall_memories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<RecallQuery>,
) -> crate::Result<impl IntoResponse> {
    let query = params.q.unwrap_or_default();
    let limit = params.limit.unwrap_or(state.config.memory_default_limit);
    let offset = params.offset.unwrap_or(0);
    let results = state.memory.recall(&query, limit, offset).await?;
    Ok(Json(results))
}

/// GET /memory/{key} — recall a specific memory by exact key.
pub async fn read_memory_by_key(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let results = state.memory.recall(&key, 100, 0).await?;
    let entry = results
        .into_iter()
        .find(|e| e.key == key)
        .ok_or_else(|| MesoError::NotFound(format!("memory key not found: {key}")))?;
    Ok(Json(entry))
}

/// PUT /memory/{key} — update (upsert) a memory entry.
pub async fn update_memory(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Json(body): Json<UpdateMemoryRequest>,
) -> crate::Result<impl IntoResponse> {
    let category = parse_category(body.category.as_deref());
    state.memory.store(&key, &body.content, category).await?;
    Ok(StatusCode::OK)
}

/// DELETE /memory/{key} — forget a memory entry.
pub async fn delete_memory(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let removed = state.memory.forget(&key).await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(MesoError::NotFound(format!("memory key not found: {key}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/memory", post(create_memory).get(recall_memories))
            .route(
                "/memory/{key}",
                get(read_memory_by_key)
                    .put(update_memory)
                    .delete(delete_memory),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn memory_create_returns_201() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/memory")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "key": "test_key",
                    "content": "test content",
                    "category": "core"
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn memory_recall_returns_results() {
        let (_dir, state) = test_state().await;
        let app = app(state.clone());

        // Store first
        state
            .memory
            .store("recall_key", "recall content", MemoryCategory::Core)
            .await
            .unwrap();

        let req = Request::builder()
            .uri("/memory?q=recall")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let entries: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(!entries.is_empty());
        assert_eq!(entries[0]["key"], "recall_key");
    }

    #[tokio::test]
    async fn memory_read_by_key() {
        let (_dir, state) = test_state().await;
        let app = app(state.clone());

        state
            .memory
            .store("specific_key", "specific content", MemoryCategory::Daily)
            .await
            .unwrap();

        let req = Request::builder()
            .uri("/memory/specific_key")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let entry: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(entry["key"], "specific_key");
        assert_eq!(entry["content"], "specific content");
    }

    #[tokio::test]
    async fn memory_update_returns_200() {
        let (_dir, state) = test_state().await;
        let app = app(state.clone());

        state
            .memory
            .store("upd_key", "old content", MemoryCategory::Core)
            .await
            .unwrap();

        let req = Request::builder()
            .method("PUT")
            .uri("/memory/upd_key")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "content": "new content",
                    "category": "daily"
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn memory_delete_returns_204() {
        let (_dir, state) = test_state().await;
        let app = app(state.clone());

        state
            .memory
            .store("del_key", "to delete", MemoryCategory::Core)
            .await
            .unwrap();

        let req = Request::builder()
            .method("DELETE")
            .uri("/memory/del_key")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn memory_recall_with_offset() {
        let (_dir, state) = test_state().await;
        let app = app(state.clone());

        // Store 3 entries
        for i in 0..3 {
            state
                .memory
                .store(
                    &format!("okey{i}"),
                    &format!("offset test {i}"),
                    MemoryCategory::Core,
                )
                .await
                .unwrap();
        }

        let req = Request::builder()
            .uri("/memory?q=offset&limit=10&offset=1")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let entries: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn memory_not_found_returns_404() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/memory/nonexistent_key")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
