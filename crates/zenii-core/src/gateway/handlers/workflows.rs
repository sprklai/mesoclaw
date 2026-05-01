use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::gateway::state::AppState;
use crate::workflows::Workflow;
use crate::{Result, ZeniiError};

const LIST_DEFAULT_LIMIT: usize = 50;
const LIST_MAX_LIMIT: usize = 200;

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

#[derive(Debug, Deserialize)]
pub struct ListWorkflowsQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListWorkflowsResponse {
    pub workflows: Vec<Workflow>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

pub async fn list_workflows(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListWorkflowsQuery>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    let limit = query
        .limit
        .unwrap_or(LIST_DEFAULT_LIMIT)
        .min(LIST_MAX_LIMIT);
    let offset = query.offset.unwrap_or(0);

    let mut all = registry.list();
    // Stable ordering by id so pagination is deterministic
    all.sort_by(|a, b| a.id.cmp(&b.id));
    let total = all.len();
    let workflows = all.into_iter().skip(offset).take(limit).collect();

    Ok(Json(ListWorkflowsResponse {
        workflows,
        total,
        offset,
        limit,
    }))
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

    let deleted = registry.delete(&id)?;
    if !deleted {
        return Err(ZeniiError::NotFound(format!("workflow '{id}' not found")));
    }
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
    // Verify the workflow exists before fetching history
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    if registry.get(&id).is_none() {
        return Err(ZeniiError::NotFound(format!("workflow '{id}' not found")));
    }

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

    // P.1 — delete unknown workflow returns 404
    #[cfg(feature = "workflows")]
    #[tokio::test]
    async fn delete_unknown_workflow_returns_404() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state_with_workflows().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/workflows/does-not-exist")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // P.2 — list workflows returns paginated response with total field
    #[cfg(feature = "workflows")]
    #[tokio::test]
    async fn list_workflows_pagination() {
        use crate::workflows::{FailurePolicy, StepType, Workflow, WorkflowStep};

        let (_dir, state) = crate::gateway::handlers::tests::test_state_with_workflows().await;

        // Pre-populate with 3 workflows directly via the registry
        {
            let registry = state.workflow_registry.as_ref().unwrap();
            for i in 1..=3_u32 {
                registry
                    .save(Workflow {
                        id: format!("wf-{i:02}"),
                        name: format!("Workflow {i}"),
                        description: format!("desc {i}"),
                        schedule: None,
                        steps: vec![WorkflowStep {
                            name: "s1".into(),
                            step_type: StepType::Delay { seconds: 1 },
                            depends_on: vec![],
                            retry: None,
                            failure_policy: FailurePolicy::Stop,
                            timeout_secs: None,
                        }],
                        layout: None,
                        created_at: "2026-01-01T00:00:00Z".into(),
                        updated_at: "2026-01-01T00:00:00Z".into(),
                    })
                    .unwrap();
            }
        }

        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/workflows?limit=2&offset=0")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 3);
        assert_eq!(json["limit"], 2);
        assert_eq!(json["offset"], 0);
        assert_eq!(json["workflows"].as_array().unwrap().len(), 2);
    }

    // P.3 — list_workflows default limit and total present when no query params
    #[cfg(feature = "workflows")]
    #[tokio::test]
    async fn list_workflows_default_limit() {
        use crate::workflows::{FailurePolicy, StepType, Workflow, WorkflowStep};

        let (_dir, state) = crate::gateway::handlers::tests::test_state_with_workflows().await;

        {
            let registry = state.workflow_registry.as_ref().unwrap();
            registry
                .save(Workflow {
                    id: "wf-single".into(),
                    name: "Single".into(),
                    description: "only one".into(),
                    schedule: None,
                    steps: vec![WorkflowStep {
                        name: "s1".into(),
                        step_type: StepType::Delay { seconds: 1 },
                        depends_on: vec![],
                        retry: None,
                        failure_policy: FailurePolicy::Stop,
                        timeout_secs: None,
                    }],
                    layout: None,
                    created_at: "2026-01-01T00:00:00Z".into(),
                    updated_at: "2026-01-01T00:00:00Z".into(),
                })
                .unwrap();
        }

        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/workflows")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["limit"], 50); // default
        assert_eq!(json["offset"], 0);
        assert_eq!(json["workflows"].as_array().unwrap().len(), 1);
    }

    // P.4 — history for unknown workflow returns 404
    #[cfg(feature = "workflows")]
    #[tokio::test]
    async fn history_unknown_workflow_returns_404() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state_with_workflows().await;
        // workflow_executor is None so if we got past the registry check it would be
        // 500; the 404 must come from the registry existence check.
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/workflows/does-not-exist/history")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
