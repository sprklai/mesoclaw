use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::ZeniiError;
use crate::gateway::state::AppState;
use crate::security::policy::ValidationResult;

#[derive(Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ExecuteToolRequest {
    pub args: serde_json::Value,
}

/// GET /tools — list all registered tools.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/tools", tag = "Tools",
    responses((status = 200, description = "List of registered tools"))
))]
pub async fn list_tools(State(state): State<Arc<AppState>>) -> crate::Result<impl IntoResponse> {
    Ok(Json(state.tools.list()))
}

/// POST /tools/{name}/execute — execute a tool by name.
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/tools/{name}/execute", tag = "Tools",
    params(("name" = String, Path, description = "Tool name")),
    request_body = ExecuteToolRequest,
    responses(
        (status = 200, description = "Tool execution result"),
        (status = 404, description = "Tool not found"),
        (status = 403, description = "Tool execution denied by security policy")
    )
))]
pub async fn execute_tool(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<ExecuteToolRequest>,
) -> crate::Result<impl IntoResponse> {
    let tool = state
        .tools
        .get(&name)
        .ok_or_else(|| ZeniiError::NotFound(format!("tool not found: {name}")))?;

    // Security policy check before execution
    match state.security.validate_tool_execution(&name, &body.args) {
        ValidationResult::Allowed => {}
        ValidationResult::NeedsApproval => {
            return Err(ZeniiError::PolicyDenied(format!(
                "tool '{name}' requires approval in supervised mode"
            )));
        }
        ValidationResult::Denied(reason) => {
            return Err(ZeniiError::PolicyDenied(reason));
        }
    }

    let result = tool.execute(body.args).await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolRegistry;
    use crate::tools::traits::{Tool, ToolResult};
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use tempfile::TempDir;
    use tower::ServiceExt;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echoes the input back"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                }
            })
        }

        async fn execute(&self, args: serde_json::Value) -> crate::Result<ToolResult> {
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("no message");
            Ok(ToolResult::ok(message))
        }
    }

    async fn test_state_with_tools(tool_list: Vec<Arc<dyn Tool>>) -> (TempDir, Arc<AppState>) {
        let (dir, base_state) = crate::gateway::handlers::tests::test_state().await;
        let registry = ToolRegistry::new();
        for tool in tool_list {
            registry.register(tool).unwrap();
        }
        let state = Arc::new(AppState {
            config: base_state.config.clone(),
            config_path: base_state.config_path.clone(),
            config_write_lock: tokio::sync::Mutex::new(()),
            db: base_state.db.clone(),
            event_bus: base_state.event_bus.clone(),
            memory: base_state.memory.clone(),
            credentials: base_state.credentials.clone(),
            security: base_state.security.clone(),
            tools: Arc::new(registry),
            session_manager: base_state.session_manager.clone(),
            agent: None,
            provider_registry: base_state.provider_registry.clone(),
            boot_context: base_state.boot_context.clone(),
            last_used_model: base_state.last_used_model.clone(),
            context_builder: base_state.context_builder.clone(),
            reasoning_engine: Arc::new(crate::ai::reasoning::ReasoningEngine::new(3)),
            prompt_strategy: base_state.prompt_strategy.clone(),
            context_injection_enabled: base_state.context_injection_enabled.clone(),
            self_evolution_enabled: base_state.self_evolution_enabled.clone(),
            soul_loader: base_state.soul_loader.clone(),
            skill_registry: base_state.skill_registry.clone(),
            user_learner: base_state.user_learner.clone(),
            plugin_registry: base_state.plugin_registry.clone(),
            plugin_installer: base_state.plugin_installer.clone(),
            #[cfg(feature = "channels")]
            channel_registry: base_state.channel_registry.clone(),
            #[cfg(feature = "channels")]
            channel_router: base_state.channel_router.clone(),
            #[cfg(feature = "scheduler")]
            scheduler: base_state.scheduler.clone(),
            notification_router: None,
            coordinator: base_state.coordinator.clone(),
            #[cfg(feature = "workflows")]
            workflow_registry: None,
            #[cfg(feature = "workflows")]
            workflow_executor: None,
            #[cfg(feature = "workflows")]
            active_workflow_runs: Arc::new(dashmap::DashMap::new()),
            usage_logger: base_state.usage_logger.clone(),
            embedding_model_available: base_state.embedding_model_available.clone(),
            approval_broker: base_state.approval_broker.clone(),
            wiki: base_state.wiki.clone(),
        });
        (dir, state)
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/tools", get(list_tools))
            .route("/tools/{name}/execute", post(execute_tool))
            .with_state(state)
    }

    #[tokio::test]
    async fn list_tools_returns_array() {
        let echo: Arc<dyn Tool> = Arc::new(EchoTool);
        let (_dir, state) = test_state_with_tools(vec![echo]).await;
        let app = app(state);

        let req = Request::builder()
            .uri("/tools")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let tools: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo");
        assert_eq!(tools[0]["description"], "Echoes the input back");
        assert!(tools[0]["parameters"].is_object());
    }

    #[tokio::test]
    async fn execute_tool_returns_result() {
        let echo: Arc<dyn Tool> = Arc::new(EchoTool);
        let (_dir, state) = test_state_with_tools(vec![echo]).await;
        let app = app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/tools/echo/execute")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "args": { "message": "hello world" }
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["output"], "hello world");
        assert_eq!(result["success"], true);
    }

    #[tokio::test]
    async fn execute_unknown_tool_returns_404() {
        let (_dir, state) = test_state_with_tools(vec![]).await;
        let app = app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/tools/nonexistent/execute")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "args": {}
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn execute_tool_empty_tools_returns_404() {
        // Variant: even with an empty tools list, a valid request returns 404
        let (_dir, state) = test_state_with_tools(vec![]).await;
        let app = app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/tools/echo/execute")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "args": { "message": "test" }
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
