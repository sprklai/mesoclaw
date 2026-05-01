use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::gateway::state::AppState;
use crate::workflows::Workflow;
use crate::{Result, ZeniiError};

/// Maximum allowed description length in bytes (Issue 5).
const MAX_DESCRIPTION_BYTES: usize = 4000;

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateWorkflowRequest {
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerateWorkflowResponse {
    pub toml: String,
    pub confidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clarifying_question: Option<String>,
    /// Whether the workflow was saved to the database.
    /// `false` when confidence is low — the TOML is returned as a preview only.
    pub saved: bool,
}

pub async fn generate_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateWorkflowRequest>,
) -> Result<impl IntoResponse> {
    // Issue 5: reject overlong descriptions before touching the LLM.
    if req.description.len() > MAX_DESCRIPTION_BYTES {
        return Err(ZeniiError::Validation(format!(
            "description too long: {} bytes (max {MAX_DESCRIPTION_BYTES})",
            req.description.len()
        )));
    }

    let generator = state
        .workflow_generator
        .as_ref()
        .ok_or_else(|| ZeniiError::Agent("no agent configured: workflow generation requires an AI provider to be configured".into()))?;

    let result = generator.generate(&req.description).await?;

    let is_low = result.confidence == crate::workflows::generator::Confidence::Low;

    // Issue 7: only save when confidence is High.
    // Issue 8: validate via TOML round-trip before saving (non-empty steps already
    //          enforced by generator; TOML parse here catches any structural gaps).
    let saved = if !is_low {
        if let Some(ref wf) = result.workflow {
            if let Some(ref registry) = state.workflow_registry {
                // Re-parse TOML to validate (catches serialization edge cases).
                let _: Workflow = toml::from_str(&result.toml).map_err(|e| {
                    ZeniiError::Workflow(format!("Generated workflow is invalid: {e}"))
                })?;
                registry.save(wf.clone())?;
                let _ = state
                    .event_bus
                    .publish(crate::event_bus::AppEvent::WorkflowsChanged);
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    Ok(Json(GenerateWorkflowResponse {
        toml: result.toml,
        confidence: if is_low { "low".to_string() } else { "high".to_string() },
        clarifying_question: result.clarifying_question,
        saved,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub toml_content: String,
}

pub async fn create_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    let workflow: Workflow = toml::from_str(&req.toml_content)?;
    registry.save(workflow.clone())?;
    let _ = state
        .event_bus
        .publish(crate::event_bus::AppEvent::WorkflowsChanged);

    Ok((StatusCode::CREATED, Json(workflow)))
}

pub async fn list_workflows(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    Ok(Json(registry.list()))
}

pub async fn get_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    registry
        .get(&id)
        .map(Json)
        .ok_or_else(|| ZeniiError::NotFound(format!("workflow '{id}' not found")))
}

pub async fn update_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    let mut workflow: Workflow = toml::from_str(&req.toml_content)?;

    // Validate ID in TOML matches path ID
    if workflow.id != id {
        return Err(ZeniiError::Validation(format!(
            "workflow ID in TOML ('{}') does not match path ID ('{id}')",
            workflow.id
        )));
    }

    // Verify workflow exists
    let existing = registry
        .get(&id)
        .ok_or_else(|| ZeniiError::NotFound(format!("workflow '{id}' not found")))?;

    // Preserve created_at, update updated_at
    workflow.created_at = existing.created_at;
    workflow.updated_at = chrono::Utc::now().to_rfc3339();

    registry.save(workflow.clone())?;
    let _ = state
        .event_bus
        .publish(crate::event_bus::AppEvent::WorkflowsChanged);
    Ok(Json(workflow))
}

pub async fn get_workflow_raw(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    registry
        .get_raw_toml(&id)
        .ok_or_else(|| ZeniiError::NotFound(format!("workflow '{id}' not found")))
}

pub async fn delete_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    registry.delete(&id)?;
    let _ = state
        .event_bus
        .publish(crate::event_bus::AppEvent::WorkflowsChanged);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn run_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    let workflow = registry
        .get(&id)
        .ok_or_else(|| ZeniiError::NotFound(format!("workflow '{id}' not found")))?;

    let executor = state
        .workflow_executor
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow executor not initialized".into()))?
        .clone();

    let tools = state.tools.clone();
    let event_bus = state.event_bus.clone();
    let active_runs = state.active_workflow_runs.clone();
    let state_for_llm = state.clone();

    // B.1: Generate run_id before spawning so we can key by it
    let run_id = uuid::Uuid::new_v4().to_string();
    let run_id_clone = run_id.clone();
    let workflow_id = id.clone();

    let handle = tokio::spawn(async move {
        let _result = executor
            .execute_with_id(
                run_id_clone.clone(),
                &workflow,
                &tools,
                event_bus.as_ref(),
                None,
                Some(&state_for_llm),
            )
            .await;
        active_runs.remove(&run_id_clone);
    });

    // B.1: Key by run_id, not workflow_id
    state
        .active_workflow_runs
        .insert(run_id.clone(), handle.abort_handle());

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "workflow_id": workflow_id, "run_id": run_id })),
    ))
}

pub async fn cancel_workflow_run(
    State(state): State<Arc<AppState>>,
    Path((workflow_id, run_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    if let Some((_, handle)) = state.active_workflow_runs.remove(&run_id) {
        handle.abort();

        // B.2: Persist "cancelled" status to DB and emit terminal event
        if let Some(ref executor) = state.workflow_executor {
            let completed_at = chrono::Utc::now().to_rfc3339();
            let _ = executor
                .persist_run_end(&run_id, "cancelled", None, &completed_at)
                .await;
        }
        let _ = state
            .event_bus
            .publish(crate::event_bus::AppEvent::WorkflowCompleted {
                workflow_id: workflow_id.clone(),
                run_id: run_id.clone(),
                status: "cancelled".into(),
            });

        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ZeniiError::NotFound(format!(
            "no active run '{run_id}' for workflow '{workflow_id}'"
        )))
    }
}

pub async fn workflow_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let executor = state
        .workflow_executor
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow executor not initialized".into()))?;

    let history = executor.get_history(&id).await?;
    Ok(Json(history))
}

pub async fn get_run_details(
    State(state): State<Arc<AppState>>,
    Path((_, run_id)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    let executor = state
        .workflow_executor
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow executor not initialized".into()))?;

    executor
        .get_run(&run_id)
        .await?
        .map(Json)
        .ok_or_else(|| ZeniiError::NotFound(format!("run '{run_id}' not found")))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    // 5.42 — create workflow returns error when registry not initialized
    #[tokio::test]
    async fn create_workflow_no_registry() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/workflows")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"toml_content":"id = \"test\"\nname = \"Test\"\ndescription = \"t\"\n\n[[steps]]\nname = \"s1\"\ntype = \"delay\"\nseconds = 1"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 5.43 — list workflows returns error when registry not initialized
    #[tokio::test]
    async fn list_workflows_no_registry() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/workflows")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 5.44 — get workflow returns error when registry not initialized
    #[tokio::test]
    async fn get_workflow_no_registry() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/workflows/nonexistent")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 5.45 — delete workflow returns error when registry not initialized
    #[tokio::test]
    async fn delete_workflow_no_registry() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/workflows/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 5.46 — run workflow returns error when executor not initialized
    #[tokio::test]
    async fn run_workflow_no_executor() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/workflows/test/run")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 5.47 — workflow history returns error when executor not initialized
    #[tokio::test]
    async fn workflow_history_no_executor() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/workflows/test/history")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 5.48 — get run details returns error when executor not initialized
    #[tokio::test]
    async fn get_run_details_no_executor() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/workflows/test/runs/run1")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 5.49 — generate workflow returns 503 when no AI provider is configured
    #[tokio::test]
    async fn test_generate_workflow_no_generator() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/workflows/generate")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"description":"check my disk"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    // 5.50 — generate workflow rejects description > 4000 bytes with HTTP 400
    #[tokio::test]
    async fn test_generate_workflow_description_too_long() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        // Build a description that exceeds 4000 bytes.
        let long_desc = "a".repeat(4001);
        let body = serde_json::json!({ "description": long_desc }).to_string();

        let req = Request::builder()
            .method("POST")
            .uri("/workflows/generate")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
