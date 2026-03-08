use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::gateway::state::AppState;

/// GET /config — return the current AppConfig with secrets redacted and paths resolved.
pub async fn get_config(State(state): State<Arc<AppState>>) -> crate::Result<impl IntoResponse> {
    let mut config_value = serde_json::to_value(state.config.as_ref())?;
    if let Some(obj) = config_value.as_object_mut() {
        // Redact secrets
        obj.insert("gateway_auth_token".to_string(), serde_json::Value::Null);

        // Resolve None paths to their actual defaults so the UI shows real values
        let default_data_dir = crate::config::default_data_dir();
        let data_dir = state
            .config
            .data_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| default_data_dir.clone());

        if state.config.data_dir.is_none() {
            obj.insert(
                "data_dir".to_string(),
                serde_json::Value::String(default_data_dir.to_string_lossy().into()),
            );
        }
        if state.config.db_path.is_none() {
            obj.insert(
                "db_path".to_string(),
                serde_json::Value::String(
                    default_data_dir
                        .join("mesoclaw.db")
                        .to_string_lossy()
                        .into(),
                ),
            );
        }
        if state.config.memory_db_path.is_none() {
            obj.insert(
                "memory_db_path".to_string(),
                serde_json::Value::String(
                    default_data_dir
                        .join("memory_vec.db")
                        .to_string_lossy()
                        .into(),
                ),
            );
        }
        if state.config.identity_dir.is_none() {
            obj.insert(
                "identity_dir".to_string(),
                serde_json::Value::String(data_dir.join("identity").to_string_lossy().into()),
            );
        }
        if state.config.skills_dir.is_none() {
            obj.insert(
                "skills_dir".to_string(),
                serde_json::Value::String(data_dir.join("skills").to_string_lossy().into()),
            );
        }
    }
    Ok(Json(config_value))
}

/// PUT /config — accept partial JSON config update. For Phase 3 this acknowledges the update
/// without persisting (full config persistence is deferred to a later phase).
pub async fn update_config(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> crate::Result<impl IntoResponse> {
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "acknowledged",
            "fields": body
        })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tempfile::TempDir;
    use tower::ServiceExt;

    /// Custom test_state that sets gateway_auth_token to verify redaction.
    async fn test_state() -> (TempDir, Arc<AppState>) {
        let (dir, base_state) = crate::gateway::handlers::tests::test_state().await;
        // Override config to set a non-None auth token for redaction tests
        let mut config = AppConfig {
            gateway_cors_origins: base_state.config.gateway_cors_origins.clone(),
            ..Default::default()
        };
        config.gateway_auth_token = Some("super_secret_token".into());
        let state = Arc::new(AppState {
            config: Arc::new(config),
            db: base_state.db.clone(),
            event_bus: base_state.event_bus.clone(),
            memory: base_state.memory.clone(),
            credentials: base_state.credentials.clone(),
            security: base_state.security.clone(),
            tools: Arc::new(crate::tools::ToolRegistry::new()),
            session_manager: base_state.session_manager.clone(),
            agent: None,
            provider_registry: base_state.provider_registry.clone(),
            soul_loader: base_state.soul_loader.clone(),
            skill_registry: base_state.skill_registry.clone(),
            user_learner: base_state.user_learner.clone(),
            #[cfg(feature = "channels")]
            channel_registry: base_state.channel_registry.clone(),
        });
        (dir, state)
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/config", get(get_config).put(update_config))
            .with_state(state)
    }

    #[tokio::test]
    async fn get_config_returns_200() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/config")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("gateway_host").is_some());
        assert!(json.get("gateway_port").is_some());
    }

    #[tokio::test]
    async fn get_config_redacts_secrets() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/config")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["gateway_auth_token"].is_null(),
            "gateway_auth_token should be redacted to null"
        );
    }

    #[tokio::test]
    async fn put_config_updates_fields() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("PUT")
            .uri("/config")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "log_level": "debug"
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "acknowledged");
        assert_eq!(json["fields"]["log_level"], "debug");
    }
}
