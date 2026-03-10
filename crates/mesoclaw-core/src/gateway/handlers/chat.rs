use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::ai::context::ContextEngine;
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
    // Build context: history + augmented preamble via ContextBuilder
    let (history, preamble) = state
        .context_builder
        .build(req.session_id.as_deref(), &req.prompt)
        .await?;

    // Also compose ContextEngine preamble for boot context / cached summaries
    let ctx_enabled = state
        .context_injection_enabled
        .load(std::sync::atomic::Ordering::Relaxed);
    let context_engine = ContextEngine::new(state.db.clone(), state.config.clone(), ctx_enabled);
    let (message_count, last_message_at, summary) = if let Some(ref sid) = req.session_id {
        state
            .session_manager
            .get_context_info(sid)
            .await
            .unwrap_or((0, None, None))
    } else {
        (0, None, None)
    };
    let level = context_engine.determine_context_level(
        message_count,
        last_message_at.as_ref(),
        summary.is_some(),
        false,
    );
    let model_display = req.model.as_deref().unwrap_or("default");
    let engine_preamble = context_engine
        .compose(
            &level,
            &state.boot_context,
            model_display,
            req.session_id.as_deref(),
            summary.as_deref(),
        )
        .await?;

    // Merge preambles: ContextBuilder (identity + memory + user) + ContextEngine (boot + summaries)
    let merged_preamble = format!("{preamble}\n\n{engine_preamble}");

    let agent = resolve_agent(req.model.as_deref(), &state, None, Some(&merged_preamble)).await?;

    // If session_id provided, store the user message
    if let Some(ref sid) = req.session_id {
        let _ = state
            .session_manager
            .append_message(sid, "user", &req.prompt)
            .await;
    }

    // Use reasoning engine for multi-turn continuity with autonomous reasoning
    let chat_result = state
        .reasoning_engine
        .chat(&agent, &req.prompt, history)
        .await?;
    let response = chat_result.response;

    // Auto-extract facts from the conversation
    if let Some(ref sid) = req.session_id {
        let _ = state
            .context_builder
            .extract_facts(&req.prompt, &response, Some(sid))
            .await;
    }

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
