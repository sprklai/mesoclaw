use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::gateway::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct SetCredentialRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialExistsResponse {
    pub exists: bool,
}

/// POST /credentials -- set a credential.
pub async fn set_credential(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetCredentialRequest>,
) -> crate::Result<impl IntoResponse> {
    state.credentials.set(&req.key, &req.value).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /credentials -- list stored credential keys (names only, never values).
pub async fn list_credentials(
    State(state): State<Arc<AppState>>,
) -> crate::Result<impl IntoResponse> {
    let keys = state.credentials.list().await?;
    Ok(Json(keys))
}

/// DELETE /credentials/{key} -- remove a credential.
pub async fn delete_credential(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let deleted = state.credentials.delete(&key).await?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

/// GET /credentials/{key}/value -- get a credential value (for local UI reveal).
pub async fn get_credential_value(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let value = state.credentials.get(&key).await?;
    match value {
        Some(v) => Ok(Json(serde_json::json!({ "key": key, "value": v }))),
        None => Err(crate::MesoError::NotFound(format!("credential '{key}'"))),
    }
}

/// GET /credentials/{key}/exists -- check if a credential exists (bool, no value).
pub async fn credential_exists(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> crate::Result<impl IntoResponse> {
    let value = state.credentials.get(&key).await?;
    Ok(Json(CredentialExistsResponse {
        exists: value.is_some(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{delete, get, post};
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn test_state() -> (TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/credentials", post(set_credential).get(list_credentials))
            .route("/credentials/{key}", delete(delete_credential))
            .route("/credentials/{key}/exists", get(credential_exists))
            .with_state(state)
    }

    #[tokio::test]
    async fn set_credential_test() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/credentials")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&SetCredentialRequest {
                    key: "api_key:openai".into(),
                    value: "sk-test".into(),
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_credential_keys() {
        let (_dir, state) = test_state().await;
        state
            .credentials
            .set("api_key:openai", "sk-test")
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri("/credentials")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let keys: Vec<String> = serde_json::from_slice(&body).unwrap();
        assert!(keys.contains(&"api_key:openai".to_string()));
    }

    #[tokio::test]
    async fn delete_credential_test() {
        let (_dir, state) = test_state().await;
        state.credentials.set("api_key:test", "val").await.unwrap();

        let app = app(state);
        let req = Request::builder()
            .method("DELETE")
            .uri("/credentials/api_key:test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["deleted"], true);
    }

    #[tokio::test]
    async fn credential_exists_test() {
        let (_dir, state) = test_state().await;
        state
            .credentials
            .set("api_key:openai", "sk-test")
            .await
            .unwrap();

        let router = app(state.clone());

        // Exists
        let req = Request::builder()
            .uri("/credentials/api_key:openai/exists")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let result: CredentialExistsResponse = serde_json::from_slice(&body).unwrap();
        assert!(result.exists);

        // Does not exist
        let router = app(state);
        let req = Request::builder()
            .uri("/credentials/api_key:missing/exists")
            .body(Body::empty())
            .unwrap();
        let resp = router.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let result: CredentialExistsResponse = serde_json::from_slice(&body).unwrap();
        assert!(!result.exists);
    }
}
