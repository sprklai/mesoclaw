use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::gateway::state::AppState;
use crate::security::permissions::{PermissionResolver, PermissionState, ToolPermissionInfo};

#[derive(Serialize, Deserialize)]
pub struct PermissionsResponse {
    pub surface: String,
    pub tools: Vec<ToolPermissionInfo>,
}

#[derive(Serialize)]
pub struct AllPermissionsResponse {
    pub surfaces: Vec<String>,
}

#[derive(Deserialize)]
pub struct SetPermissionRequest {
    pub state: PermissionState,
}

/// GET /permissions — list all known surfaces.
pub async fn list_surfaces(State(state): State<Arc<AppState>>) -> Json<AllPermissionsResponse> {
    let config = state.config.load();
    let mut surfaces: Vec<String> = config.tool_permissions.overrides.keys().cloned().collect();

    // Always include standard surfaces
    for s in &["desktop", "cli", "tui", "telegram", "slack", "discord"] {
        if !surfaces.contains(&s.to_string()) {
            surfaces.push(s.to_string());
        }
    }
    surfaces.sort();

    Json(AllPermissionsResponse { surfaces })
}

/// GET /permissions/{surface} — list tool permissions for a surface.
pub async fn get_permissions(
    State(state): State<Arc<AppState>>,
    Path(surface): Path<String>,
) -> Json<PermissionsResponse> {
    let config = state.config.load();
    let tools =
        PermissionResolver::list_permissions(&config.tool_permissions, &surface, &state.tools);

    Json(PermissionsResponse { surface, tools })
}

/// PUT /permissions/{surface}/{tool} — set a permission override.
pub async fn set_permission(
    State(state): State<Arc<AppState>>,
    Path((surface, tool)): Path<(String, String)>,
    Json(body): Json<SetPermissionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify tool exists
    if state.tools.get(&tool).is_none() {
        return Err((StatusCode::NOT_FOUND, format!("Tool '{tool}' not found")));
    }

    // Update config
    let _lock = state.config_write_lock.lock().await;
    let mut config = (*state.config.load_full()).clone();
    config
        .tool_permissions
        .overrides
        .entry(surface)
        .or_default()
        .insert(tool, body.state);

    state.config.store(Arc::new(config.clone()));

    // Persist to disk
    if let Err(e) = crate::config::save_config(&state.config_path, &config) {
        tracing::warn!("Failed to persist config: {e}");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /permissions/{surface}/{tool} — remove an override (fall back to default).
pub async fn delete_permission(
    State(state): State<Arc<AppState>>,
    Path((surface, tool)): Path<(String, String)>,
) -> StatusCode {
    let _lock = state.config_write_lock.lock().await;
    let mut config = (*state.config.load_full()).clone();

    if let Some(surface_overrides) = config.tool_permissions.overrides.get_mut(&surface) {
        surface_overrides.remove(&tool);
        if surface_overrides.is_empty() {
            config.tool_permissions.overrides.remove(&surface);
        }
    }

    state.config.store(Arc::new(config.clone()));

    if let Err(e) = crate::config::save_config(&state.config_path, &config) {
        tracing::warn!("Failed to persist config: {e}");
    }

    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use crate::gateway::routes::build_router;

    async fn test_state() -> (tempfile::TempDir, std::sync::Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    use super::*;

    // P19.19 — GET /permissions/{surface} returns tool list
    #[tokio::test]
    async fn get_permissions_surface() {
        let (_dir, state) = test_state().await;

        // Register a tool so there's something to list
        state
            .tools
            .register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))
            .unwrap();

        let app = build_router(state);
        let req = Request::builder()
            .uri("/permissions/telegram")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: PermissionsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.surface, "telegram");
        assert!(!result.tools.is_empty());
    }

    // P19.20 — PUT /permissions/{surface}/{tool} sets override
    #[tokio::test]
    async fn put_permission_override() {
        let (_dir, state) = test_state().await;
        state
            .tools
            .register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))
            .unwrap();

        let app = build_router(state);
        let req = Request::builder()
            .method("PUT")
            .uri("/permissions/telegram/system_info")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"state":"denied"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    // P19.21 — DELETE /permissions/{surface}/{tool} removes override
    #[tokio::test]
    async fn delete_permission_override() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .method("DELETE")
            .uri("/permissions/telegram/system_info")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
