use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::gateway::state::AppState;
use crate::workflows::Workflow;
use crate::{Result, ZeniiError};

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

    Ok((StatusCode::CREATED, Json(workflow)))
}

pub async fn list_workflows(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse> {
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

pub async fn delete_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let registry = state
        .workflow_registry
        .as_ref()
        .ok_or_else(|| ZeniiError::Workflow("workflow feature not initialized".into()))?;

    registry.delete(&id)?;
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
        .ok_or_else(|| ZeniiError::Workflow("workflow executor not initialized".into()))?;

    let run = executor
        .execute(&workflow, &state.tools, state.event_bus.as_ref())
        .await?;

    Ok((StatusCode::ACCEPTED, Json(run)))
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
}
