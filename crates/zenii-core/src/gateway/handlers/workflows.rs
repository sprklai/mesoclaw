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
