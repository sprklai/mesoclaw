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
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ChatResponse {
    pub response: String,
    pub session_id: Option<String>,
}

#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/chat", tag = "Chat",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Chat response", body = ChatResponse),
        (status = 502, description = "Agent error", body = Object),
    )
))]
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
    let self_evo = state
        .self_evolution_enabled
        .load(std::sync::atomic::Ordering::Relaxed);
    let mut context_engine =
        ContextEngine::new(state.db.clone(), state.config.load_full(), ctx_enabled)
            .with_skill_registry(state.skill_registry.clone())
            .with_self_evolution(self_evo);
    #[cfg(feature = "channels")]
    {
        context_engine = context_engine.with_channel_registry(state.channel_registry.clone());
    }
    #[cfg(feature = "scheduler")]
    if let Some(ref sched) = state.scheduler {
        context_engine = context_engine.with_scheduler(sched.clone());
    }
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
            Some(&req.prompt),
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
            .await
            .inspect_err(|e| {
                tracing::warn!("Failed to persist user message for session {sid}: {e}");
            });
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
            .await
            .inspect_err(|e| {
                tracing::warn!("Failed to persist assistant message for session {sid}: {e}");
            });
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

    // 3.4.1 — chat post with no API key returns 500 (credential error)
    #[tokio::test]
    async fn chat_no_agent_returns_credential_error() {
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
        // Default model is seeded (anthropic:claude-sonnet-4-6) but no API key exists,
        // so resolve_agent fails with a Credential error (500).
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
