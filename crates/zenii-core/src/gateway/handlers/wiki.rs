use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::gateway::state::AppState;

// ── Request / query types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Deserialize)]
pub struct IngestRequest {
    pub content: String,
    pub filename: String,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /wiki — list all wiki pages.
pub async fn list_wiki_pages(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.wiki.list_pages() {
        Ok(pages) => (StatusCode::OK, Json(serde_json::json!(pages))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /wiki/{slug} — fetch a single wiki page by slug.
pub async fn get_wiki_page(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.wiki.get_page(&slug) {
        Ok(Some(page)) => (StatusCode::OK, Json(serde_json::json!(page))).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /wiki/search?q= — full-text search over wiki pages.
pub async fn search_wiki_pages(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = params.q.as_deref().unwrap_or("");
    match state.wiki.search_pages(q) {
        Ok(pages) => (StatusCode::OK, Json(serde_json::json!(pages))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /wiki/ingest — ingest a raw source document into the wiki.
pub async fn ingest_wiki_source(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IngestRequest>,
) -> impl IntoResponse {
    match state.wiki.ingest(&body.filename, &body.content) {
        Ok(page) => (
            StatusCode::OK,
            Json(serde_json::json!({"slug": page.slug, "status": "created"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /wiki/sync — sync compiled wiki pages into the memory store.
pub async fn sync_wiki_to_memory(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.wiki.sync_to_memory(state.memory.as_ref()).await {
        Ok(count) => {
            (StatusCode::OK, Json(serde_json::json!({"synced": count}))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /wiki/graph — return wiki knowledge graph (nodes + edges).
pub async fn get_wiki_graph(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.wiki.graph() {
        Ok(graph) => (StatusCode::OK, Json(serde_json::json!(graph))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

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
            .route("/wiki", get(list_wiki_pages))
            .route("/wiki/search", get(search_wiki_pages))
            .route("/wiki/{slug}", get(get_wiki_page))
            .route("/wiki/ingest", post(ingest_wiki_source))
            .route("/wiki/sync", post(sync_wiki_to_memory))
            .route("/wiki/graph", get(get_wiki_graph))
            .with_state(state)
    }

    /// H1: GET /wiki with no pages → 200 with empty array `[]`
    #[tokio::test]
    async fn wiki_list_empty_returns_200_empty_array() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(pages.is_empty(), "expected empty array");
    }

    /// H2: GET /wiki → 200 with a JSON array (even if empty during stub phase)
    #[tokio::test]
    async fn wiki_list_returns_pages() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        // Body must be a valid JSON array
        let _pages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    }

    /// H3: GET /wiki/{slug} for a page that exists → 200
    #[tokio::test]
    async fn wiki_get_existing_returns_200() {
        let (_dir, state) = test_state().await;

        // Seed a page via ingest so the slug exists
        let ingest_body = serde_json::json!({
            "filename": "test-page.md",
            "content": "---\ntitle: \"Test Page\"\ntype: concept\ntags: []\nsources: []\nupdated: 2026-01-01\n---\n\n## TLDR\nTest page for handler tests.\n\n## Body\nContent here.\n"
        });
        let seed_req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&ingest_body).unwrap()))
            .unwrap();
        let seed_resp = app(state.clone()).oneshot(seed_req).await.unwrap();
        assert_eq!(seed_resp.status(), StatusCode::OK, "ingest seed must succeed");

        let req = Request::builder()
            .uri("/wiki/test-page")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// H4: GET /wiki/{slug} for a page that does not exist → 404
    #[tokio::test]
    async fn wiki_get_missing_returns_404() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/no-such-page")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// H5: GET /wiki/search?q=test → 200
    #[tokio::test]
    async fn wiki_search_returns_200() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/search?q=test")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// H6: POST /wiki/ingest with a valid JSON body → 200
    #[tokio::test]
    async fn wiki_ingest_returns_200() {
        let (_dir, state) = test_state().await;

        let body = serde_json::to_string(&serde_json::json!({
            "content": "# Hello\nThis is a test document.",
            "filename": "hello.md"
        }))
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/ingest")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// H7: POST /wiki/sync → 200 and body contains a `synced` key
    #[tokio::test]
    async fn wiki_sync_returns_200_with_synced_count() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .method("POST")
            .uri("/wiki/sync")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            val.get("synced").is_some(),
            "response body must contain 'synced' key"
        );
    }

    /// H8: GET /wiki/graph → 200 and body contains `nodes` and `edges` keys
    #[tokio::test]
    async fn wiki_graph_returns_200_with_nodes_edges() {
        let (_dir, state) = test_state().await;

        let req = Request::builder()
            .uri("/wiki/graph")
            .body(Body::empty())
            .unwrap();

        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            val.get("nodes").is_some(),
            "response body must contain 'nodes' key"
        );
        assert!(
            val.get("edges").is_some(),
            "response body must contain 'edges' key"
        );
    }
}
