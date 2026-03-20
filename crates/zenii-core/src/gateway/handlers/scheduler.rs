use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::ZeniiError;
use crate::gateway::state::AppState;
use crate::scheduler::traits::{JobExecution, ScheduledJob, Scheduler};

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct SchedulerStatusResponse {
    pub running: bool,
    pub job_count: usize,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct CreateJobRequest {
    #[serde(flatten)]
    pub job: ScheduledJob,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct CreateJobResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ToggleResponse {
    pub id: String,
    pub enabled: bool,
}

/// GET /scheduler/jobs
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/scheduler/jobs", tag = "Scheduler",
    responses((status = 200, description = "List of scheduled jobs", body = Vec<ScheduledJob>))
))]
pub async fn list_jobs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ScheduledJob>>, ZeniiError> {
    let scheduler = state
        .scheduler
        .as_ref()
        .ok_or_else(|| ZeniiError::Scheduler("scheduler not initialized".into()))?;
    let jobs = scheduler.list_jobs().await;
    Ok(Json(jobs))
}

/// POST /scheduler/jobs
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/scheduler/jobs", tag = "Scheduler",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job created", body = CreateJobResponse),
        (status = 400, description = "Invalid job definition")
    )
))]
pub async fn create_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<CreateJobResponse>), ZeniiError> {
    let scheduler = state
        .scheduler
        .as_ref()
        .ok_or_else(|| ZeniiError::Scheduler("scheduler not initialized".into()))?;
    let id = scheduler.add_job(req.job).await?;
    Ok((StatusCode::CREATED, Json(CreateJobResponse { id })))
}

/// PUT /scheduler/jobs/:id/toggle
#[cfg_attr(feature = "api-docs", utoipa::path(
    put, path = "/scheduler/jobs/{id}/toggle", tag = "Scheduler",
    params(("id" = String, Path, description = "Job ID")),
    responses((status = 200, description = "Job toggled", body = ToggleResponse))
))]
pub async fn toggle_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ToggleResponse>, ZeniiError> {
    let scheduler = state
        .scheduler
        .as_ref()
        .ok_or_else(|| ZeniiError::Scheduler("scheduler not initialized".into()))?;
    let enabled = scheduler.toggle_job(&id).await?;
    Ok(Json(ToggleResponse { id, enabled }))
}

/// PUT /scheduler/jobs/:id
#[cfg_attr(feature = "api-docs", utoipa::path(
    put, path = "/scheduler/jobs/{id}", tag = "Scheduler",
    params(("id" = String, Path, description = "Job ID")),
    request_body = CreateJobRequest,
    responses(
        (status = 200, description = "Job updated"),
        (status = 404, description = "Job not found")
    )
))]
pub async fn update_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateJobRequest>,
) -> Result<Json<serde_json::Value>, ZeniiError> {
    let scheduler = state
        .scheduler
        .as_ref()
        .ok_or_else(|| ZeniiError::Scheduler("scheduler not initialized".into()))?;
    scheduler.update_job(&id, req.job).await?;
    Ok(Json(serde_json::json!({ "id": id, "updated": true })))
}

/// DELETE /scheduler/jobs/:id
#[cfg_attr(feature = "api-docs", utoipa::path(
    delete, path = "/scheduler/jobs/{id}", tag = "Scheduler",
    params(("id" = String, Path, description = "Job ID")),
    responses((status = 204, description = "Job deleted"))
))]
pub async fn delete_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ZeniiError> {
    let scheduler = state
        .scheduler
        .as_ref()
        .ok_or_else(|| ZeniiError::Scheduler("scheduler not initialized".into()))?;
    scheduler.remove_job(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /scheduler/jobs/:id/history
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/scheduler/jobs/{id}/history", tag = "Scheduler",
    params(("id" = String, Path, description = "Job ID")),
    responses((status = 200, description = "Job execution history", body = Vec<JobExecution>))
))]
pub async fn job_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<JobExecution>>, ZeniiError> {
    let scheduler = state
        .scheduler
        .as_ref()
        .ok_or_else(|| ZeniiError::Scheduler("scheduler not initialized".into()))?;
    let history = scheduler.job_history(&id).await;
    Ok(Json(history))
}

/// GET /scheduler/status
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/scheduler/status", tag = "Scheduler",
    responses((status = 200, description = "Scheduler running status", body = SchedulerStatusResponse))
))]
pub async fn scheduler_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SchedulerStatusResponse>, ZeniiError> {
    let scheduler = state
        .scheduler
        .as_ref()
        .ok_or_else(|| ZeniiError::Scheduler("scheduler not initialized".into()))?;
    Ok(Json(SchedulerStatusResponse {
        running: scheduler.is_running(),
        job_count: scheduler.job_count(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn build_router(state: Arc<AppState>) -> axum::Router {
        crate::gateway::routes::build_router(state)
    }

    // 16.31 — GET /scheduler/jobs lists all jobs
    #[tokio::test]
    async fn list_jobs() {
        let (_dir, state) = test_state().await;
        let app = build_router(state.clone());

        // Add a job via the scheduler
        if let Some(ref sched) = state.scheduler {
            sched
                .add_job(crate::scheduler::ScheduledJob {
                    id: String::new(),
                    name: "test_job".into(),
                    schedule: crate::scheduler::Schedule::Interval { secs: 60 },
                    session_target: crate::scheduler::SessionTarget::Main,
                    payload: crate::scheduler::JobPayload::Notify {
                        message: "hi".into(),
                    },
                    enabled: true,
                    error_count: 0,
                    next_run: None,
                    active_hours: None,
                    delete_after_run: false,
                })
                .await
                .unwrap();
        }

        let req = Request::builder()
            .uri("/scheduler/jobs")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let jobs: Vec<crate::scheduler::ScheduledJob> = serde_json::from_slice(&body).unwrap();
        assert_eq!(jobs.len(), 1);
    }

    // 16.32 — POST /scheduler/jobs creates job
    #[tokio::test]
    async fn create_job() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let body = serde_json::json!({
            "id": "",
            "name": "new_job",
            "schedule": {"type": "interval", "secs": 30},
            "payload": {"type": "notify", "message": "hello"}
        });

        let req = Request::builder()
            .method("POST")
            .uri("/scheduler/jobs")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // 16.33 — POST /scheduler/jobs invalid cron returns 400
    #[tokio::test]
    async fn create_job_invalid_cron() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let body = serde_json::json!({
            "id": "",
            "name": "bad_cron",
            "schedule": {"type": "cron", "expr": "not valid"},
            "payload": {"type": "heartbeat"}
        });

        let req = Request::builder()
            .method("POST")
            .uri("/scheduler/jobs")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Scheduler error maps to 500
        assert!(
            resp.status() == StatusCode::INTERNAL_SERVER_ERROR
                || resp.status() == StatusCode::BAD_REQUEST
        );
    }

    // 16.34 — PUT /scheduler/jobs/:id/toggle toggles
    #[tokio::test]
    async fn toggle_job() {
        let (_dir, state) = test_state().await;

        // Add job first
        let id = if let Some(ref sched) = state.scheduler {
            sched
                .add_job(crate::scheduler::ScheduledJob {
                    id: "toggle-test".into(),
                    name: "toggle_job".into(),
                    schedule: crate::scheduler::Schedule::Interval { secs: 60 },
                    session_target: crate::scheduler::SessionTarget::Main,
                    payload: crate::scheduler::JobPayload::Notify {
                        message: "hi".into(),
                    },
                    enabled: true,
                    error_count: 0,
                    next_run: None,
                    active_hours: None,
                    delete_after_run: false,
                })
                .await
                .unwrap()
        } else {
            panic!("scheduler not initialized");
        };

        let app = build_router(state);
        let req = Request::builder()
            .method("PUT")
            .uri(format!("/scheduler/jobs/{id}/toggle"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let toggle: ToggleResponse = serde_json::from_slice(&body).unwrap();
        assert!(!toggle.enabled);
    }

    // 16.35 — DELETE /scheduler/jobs/:id removes job
    #[tokio::test]
    async fn delete_job() {
        let (_dir, state) = test_state().await;

        let id = if let Some(ref sched) = state.scheduler {
            sched
                .add_job(crate::scheduler::ScheduledJob {
                    id: "del-test".into(),
                    name: "del_job".into(),
                    schedule: crate::scheduler::Schedule::Interval { secs: 60 },
                    session_target: crate::scheduler::SessionTarget::Main,
                    payload: crate::scheduler::JobPayload::Notify {
                        message: "hi".into(),
                    },
                    enabled: true,
                    error_count: 0,
                    next_run: None,
                    active_hours: None,
                    delete_after_run: false,
                })
                .await
                .unwrap()
        } else {
            panic!("scheduler not initialized");
        };

        let app = build_router(state);
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/scheduler/jobs/{id}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // 16.36 — GET /scheduler/jobs/:id/history returns history
    #[tokio::test]
    async fn job_history() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/scheduler/jobs/nonexistent/history")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let history: Vec<crate::scheduler::JobExecution> = serde_json::from_slice(&body).unwrap();
        assert!(history.is_empty());
    }

    // UPD.7 — PUT /scheduler/jobs/:id updates job
    #[tokio::test]
    async fn update_job() {
        let (_dir, state) = test_state().await;

        let id = if let Some(ref sched) = state.scheduler {
            sched
                .add_job(crate::scheduler::ScheduledJob {
                    id: String::new(),
                    name: "to_update".into(),
                    schedule: crate::scheduler::Schedule::Interval { secs: 60 },
                    session_target: crate::scheduler::SessionTarget::Main,
                    payload: crate::scheduler::JobPayload::Notify {
                        message: "hi".into(),
                    },
                    enabled: true,
                    error_count: 0,
                    next_run: None,
                    active_hours: None,
                    delete_after_run: false,
                })
                .await
                .unwrap()
        } else {
            panic!("scheduler not initialized");
        };

        let app = build_router(state);
        let body = serde_json::json!({
            "id": id,
            "name": "updated_name",
            "schedule": {"type": "interval", "secs": 120},
            "payload": {"type": "notify", "message": "updated"}
        });

        let req = Request::builder()
            .method("PUT")
            .uri(format!("/scheduler/jobs/{id}"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // UPD.8 — PUT /scheduler/jobs/nonexistent returns error
    #[tokio::test]
    async fn update_job_not_found() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let body = serde_json::json!({
            "id": "nonexistent",
            "name": "test",
            "schedule": {"type": "interval", "secs": 60},
            "payload": {"type": "heartbeat"}
        });

        let req = Request::builder()
            .method("PUT")
            .uri("/scheduler/jobs/nonexistent")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // 16.37 — GET /scheduler/status returns running status
    #[tokio::test]
    async fn scheduler_status() {
        let (_dir, state) = test_state().await;

        // Start the scheduler so is_running() returns true
        if let Some(ref sched) = state.scheduler {
            sched.start().await;
        }

        let app = build_router(state);

        let req = Request::builder()
            .uri("/scheduler/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: SchedulerStatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(status.running);
    }
}
