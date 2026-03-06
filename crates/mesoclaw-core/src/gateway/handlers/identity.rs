use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::MesoError;
use crate::gateway::state::AppState;
use crate::identity::PersonaFile;

#[derive(Serialize, Deserialize)]
pub struct IdentityListResponse {
    pub files: Vec<IdentityFileInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct IdentityFileInfo {
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

impl From<&PersonaFile> for IdentityFileInfo {
    fn from(pf: &PersonaFile) -> Self {
        Self {
            name: pf.name.clone(),
            description: pf.description.clone(),
            is_default: pf.is_default,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct IdentityFileResponse {
    pub name: String,
    pub content: String,
    pub is_default: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateIdentityRequest {
    pub content: String,
}

/// GET /identity — list all identity files
pub async fn list_identity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IdentityListResponse>, MesoError> {
    let identity = state.soul_loader.get().await;
    let mut files: Vec<IdentityFileInfo> = identity
        .files
        .values()
        .map(IdentityFileInfo::from)
        .collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(IdentityListResponse { files }))
}

/// GET /identity/{name} — get file content
pub async fn get_identity_file(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<IdentityFileResponse>, MesoError> {
    let file = state.soul_loader.get_file(&name).await?;
    Ok(Json(IdentityFileResponse {
        name: file.name,
        content: file.content,
        is_default: file.is_default,
    }))
}

/// PUT /identity/{name} — update file content
pub async fn update_identity_file(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<UpdateIdentityRequest>,
) -> Result<Json<serde_json::Value>, MesoError> {
    state.soul_loader.update_file(&name, body.content).await?;
    Ok(Json(serde_json::json!({"status": "updated"})))
}

/// POST /identity/reload — force reload all files
pub async fn reload_identity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, MesoError> {
    state.soul_loader.reload().await?;
    Ok(Json(serde_json::json!({"status": "reloaded"})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::routes::build_router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    #[tokio::test]
    async fn list_identity_files() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/identity")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: IdentityListResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.files.len(), 3);
    }

    #[tokio::test]
    async fn get_identity_file() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/identity/SOUL")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: IdentityFileResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.name, "SOUL");
        assert!(!json.content.is_empty());
    }

    #[tokio::test]
    async fn get_identity_file_not_found() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/identity/NONEXISTENT")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_identity_file() {
        let (_dir, state) = test_state().await;
        let app = build_router(state.clone());

        let req = Request::builder()
            .method("PUT")
            .uri("/identity/SOUL")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&UpdateIdentityRequest {
                    content: "New soul content".into(),
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify update
        let file = state.soul_loader.get_file("SOUL").await.unwrap();
        assert_eq!(file.content, "New soul content");
    }

    #[tokio::test]
    async fn reload_identity() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .method("POST")
            .uri("/identity/reload")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
