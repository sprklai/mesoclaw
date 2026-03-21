use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::ai::prompt::AssemblyRequest;
use crate::ai::resolve_agent;
use crate::event_bus::AppEvent;
use crate::gateway::state::AppState;
use crate::logging::UsageRecord;

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ChatRequest {
    pub prompt: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub delegation: Option<bool>,
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
    // Auto-create session when none provided (so CLI messages persist)
    let session_id = match req.session_id.clone() {
        Some(sid) => sid,
        None => {
            let title: String = req.prompt.chars().take(50).collect();
            let session = state
                .session_manager
                .create_session_with_source(&title, "api")
                .await?;
            let _ = state.event_bus.publish(AppEvent::SessionCreated {
                session_id: session.id.clone(),
                title: session.title.clone(),
                source: session.source.clone(),
            });
            session.id
        }
    };

    // Build context parts via ContextBuilder
    let (history, _memories, _user_obs) = state
        .context_builder
        .build_parts(Some(&session_id), &req.prompt)
        .await?;

    // Get conversation summary for resumed sessions
    let summary = state
        .session_manager
        .get_context_info(&session_id)
        .await
        .ok()
        .and_then(|(_, _, s)| s);

    // Assemble preamble via PromptStrategy
    let config = state.config.load_full();
    let model_display = req.model.as_deref().unwrap_or("default");
    let assembly_request = AssemblyRequest {
        boot_context: state.boot_context.clone(),
        model_display: model_display.into(),
        session_id: Some(session_id.clone()),
        user_message: Some(req.prompt.clone()),
        conversation_summary: summary,
        channel_hint: None,
        tool_count: state.tools.len(),
        skill_count: state.skill_registry.list().await.len(),
        version: config.identity_name.clone(),
    };
    let preamble = state.prompt_strategy.assemble(&assembly_request).await?;

    let agent = resolve_agent(
        req.model.as_deref(),
        &state,
        None,
        Some(&preamble),
        "desktop",
    )
    .await?;

    // Store the user message
    if let Ok(msg) = state
        .session_manager
        .append_message(&session_id, "user", &req.prompt)
        .await
        .inspect_err(|e| {
            tracing::warn!("Failed to persist user message for session {session_id}: {e}");
        })
    {
        let _ = state.event_bus.publish(AppEvent::MessageAdded {
            session_id: session_id.clone(),
            message_id: msg.id,
            role: "user".into(),
        });
    }

    // Use reasoning engine for multi-turn continuity with autonomous reasoning
    let start = std::time::Instant::now();
    let chat_result = state
        .reasoning_engine
        .chat(&agent, &req.prompt, history)
        .await?;
    let duration_ms = start.elapsed().as_millis() as u64;
    let response = chat_result.response;

    // Log usage
    let record = UsageRecord {
        timestamp: chrono::Utc::now().to_rfc3339(),
        session_id: Some(session_id.clone()),
        model_id: model_display.to_string(),
        provider_id: model_display
            .split(':')
            .next()
            .unwrap_or("unknown")
            .to_string(),
        input_tokens: chat_result.usage.input_tokens,
        output_tokens: chat_result.usage.output_tokens,
        total_tokens: chat_result.usage.total_tokens,
        cached_input_tokens: chat_result.usage.cached_input_tokens,
        tool_calls_count: agent.tool_calls_made(),
        duration_ms,
        context_level: "Full".into(),
        binary: state.usage_logger.binary_name().to_string(),
        success: true,
    };
    let logger = state.usage_logger.clone();
    tokio::spawn(async move {
        let _ = logger.log(&record).await;
    });

    // Auto-extract facts from the conversation
    let _ = state
        .context_builder
        .extract_facts(&req.prompt, &response, Some(&session_id))
        .await;

    // Store the assistant response
    if let Ok(msg) = state
        .session_manager
        .append_message(&session_id, "assistant", &response)
        .await
        .inspect_err(|e| {
            tracing::warn!("Failed to persist assistant message for session {session_id}: {e}");
        })
    {
        let _ = state.event_bus.publish(AppEvent::MessageAdded {
            session_id: session_id.clone(),
            message_id: msg.id,
            role: "assistant".into(),
        });
    }

    Ok(Json(ChatResponse {
        response,
        session_id: Some(session_id),
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
