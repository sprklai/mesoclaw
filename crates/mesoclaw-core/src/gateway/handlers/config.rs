use std::sync::Arc;

use crate::gateway::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

/// GET /config — return the current AppConfig with secrets redacted and paths resolved.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/config", tag = "Config",
    responses((status = 200, description = "Current configuration", body = Object))
))]
pub async fn get_config(State(state): State<Arc<AppState>>) -> crate::Result<impl IntoResponse> {
    let cfg = state.config.load();
    let mut config_value = serde_json::to_value(cfg.as_ref())?;
    if let Some(obj) = config_value.as_object_mut() {
        // Redact secrets
        obj.insert("gateway_auth_token".to_string(), serde_json::Value::Null);

        // Resolve None paths to their actual defaults so the UI shows real values
        let default_data_dir = crate::config::default_data_dir();
        let data_dir = cfg
            .data_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| default_data_dir.clone());

        if cfg.data_dir.is_none() {
            obj.insert(
                "data_dir".to_string(),
                serde_json::Value::String(default_data_dir.to_string_lossy().into()),
            );
        }
        if cfg.db_path.is_none() {
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
        if cfg.memory_db_path.is_none() {
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
        if cfg.identity_dir.is_none() {
            obj.insert(
                "identity_dir".to_string(),
                serde_json::Value::String(data_dir.join("identity").to_string_lossy().into()),
            );
        }
        if cfg.skills_dir.is_none() {
            obj.insert(
                "skills_dir".to_string(),
                serde_json::Value::String(data_dir.join("skills").to_string_lossy().into()),
            );
        }
    }
    // Override with runtime values (may differ from config file)
    if let Some(obj) = config_value.as_object_mut() {
        obj.insert(
            "context_injection_enabled".to_string(),
            serde_json::Value::Bool(
                state
                    .context_injection_enabled
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        );
        obj.insert(
            "self_evolution_enabled".to_string(),
            serde_json::Value::Bool(
                state
                    .self_evolution_enabled
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        );
    }

    Ok(Json(config_value))
}

/// PUT /config — accept partial JSON config update, persist to TOML, update runtime state.
#[cfg_attr(feature = "api-docs", utoipa::path(
    put, path = "/config", tag = "Config",
    request_body = Object,
    responses(
        (status = 200, description = "Config updated", body = Object),
        (status = 400, description = "Validation error", body = Object),
    )
))]
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> crate::Result<impl IntoResponse> {
    // Acquire write lock to prevent concurrent read-modify-write races
    let _lock = state.config_write_lock.lock().await;

    // Load current config from disk, merge partial update, save back
    let mut config = crate::config::load_config(&state.config_path)?;

    if let Some(obj) = body.as_object() {
        // Apply known fields
        if let Some(v) = obj
            .get("context_injection_enabled")
            .and_then(|v| v.as_bool())
        {
            config.context_injection_enabled = v;
            state
                .context_injection_enabled
                .store(v, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(v) = obj.get("self_evolution_enabled").and_then(|v| v.as_bool()) {
            config.self_evolution_enabled = v;
            state
                .self_evolution_enabled
                .store(v, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(v) = obj
            .get("context_reinject_gap_minutes")
            .and_then(|v| v.as_u64())
        {
            config.context_reinject_gap_minutes = v as u32;
        }
        if let Some(v) = obj
            .get("context_reinject_message_count")
            .and_then(|v| v.as_u64())
        {
            config.context_reinject_message_count = v as u32;
        }
        if let Some(v) = obj.get("learning_enabled").and_then(|v| v.as_bool()) {
            config.learning_enabled = v;
        }
        if let Some(v) = obj.get("agent_system_prompt") {
            config.agent_system_prompt = v.as_str().map(|s| s.to_string());
        }
        if let Some(v) = obj.get("context_strategy").and_then(|v| v.as_str()) {
            // Validate: only accept known strategy values
            match v {
                "minimal" | "balanced" | "full" => {
                    config.context_strategy = v.to_string();
                }
                _ => {
                    return Err(crate::MesoError::Validation(format!(
                        "invalid context_strategy '{v}': expected minimal, balanced, or full"
                    )));
                }
            }
        }
        if let Some(v) = obj.get("embedding_provider").and_then(|v| v.as_str()) {
            match v {
                "none" | "local" | "openai" => {
                    config.embedding_provider = v.to_string();
                }
                _ => {
                    return Err(crate::MesoError::Validation(format!(
                        "invalid embedding_provider '{v}': expected none, local, or openai"
                    )));
                }
            }
        }
        if let Some(v) = obj.get("embedding_model").and_then(|v| v.as_str()) {
            config.embedding_model = v.to_string();
        }
        // User profile & environment
        if let Some(v) = obj.get("user_name") {
            config.user_name = v.as_str().map(|s| s.to_string());
        }
        if let Some(v) = obj.get("identity_name")
            && let Some(s) = v.as_str()
        {
            config.identity_name = s.to_string();
        }
        if let Some(v) = obj.get("user_location") {
            config.user_location = v.as_str().map(|s| s.to_string());
        }
        if let Some(v) = obj.get("user_timezone") {
            config.user_timezone = v.as_str().map(|s| s.to_string());
        }
        // Channel config fields (Task 3.5)
        if let Some(v) = obj.get("telegram_dm_policy").and_then(|v| v.as_str()) {
            config.telegram_dm_policy = v.to_string();
        }
        if let Some(v) = obj
            .get("telegram_polling_timeout_secs")
            .and_then(|v| v.as_u64())
        {
            config.telegram_polling_timeout_secs = v as u32;
        }
        if let Some(v) = obj
            .get("telegram_require_group_mention")
            .and_then(|v| v.as_bool())
        {
            config.telegram_require_group_mention = v;
        }
        if let Some(v) = obj.get("telegram_retry_min_ms").and_then(|v| v.as_u64()) {
            config.telegram_retry_min_ms = v;
        }
        if let Some(v) = obj.get("telegram_retry_max_ms").and_then(|v| v.as_u64()) {
            config.telegram_retry_max_ms = v;
        }
        if let Some(v) = obj
            .get("slack_allowed_channel_ids")
            .and_then(|v| v.as_array())
        {
            config.slack_allowed_channel_ids = v
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(v) = obj
            .get("discord_allowed_guild_ids")
            .and_then(|v| v.as_array())
        {
            config.discord_allowed_guild_ids = v.iter().filter_map(|x| x.as_u64()).collect();
        }
        if let Some(v) = obj
            .get("discord_allowed_channel_ids")
            .and_then(|v| v.as_array())
        {
            config.discord_allowed_channel_ids = v.iter().filter_map(|x| x.as_u64()).collect();
        }
        // Notification routing
        if let Some(v) = obj.get("notification_routing") {
            match serde_json::from_value::<crate::notification::routing::NotificationRouting>(
                v.clone(),
            ) {
                Ok(routing) => {
                    config.notification_routing = routing;
                }
                Err(e) => {
                    return Err(crate::MesoError::Validation(format!(
                        "invalid notification_routing: {e}"
                    )));
                }
            }
        }
    }

    // Validate before saving
    config.validate();

    crate::config::save_config(&state.config_path, &config)?;

    // Swap the runtime config so all readers see the update immediately
    state.config.store(Arc::new(config));

    tracing::info!("Config updated and persisted to {:?}", state.config_path);

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "persisted",
            "fields": body
        })),
    ))
}

/// GET /config/file — return the config file path and raw TOML content.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/config/file", tag = "Config",
    responses((status = 200, description = "Config file path and TOML content", body = Object))
))]
pub async fn get_config_file(
    State(state): State<Arc<AppState>>,
) -> crate::Result<impl IntoResponse> {
    let path = state.config_path.display().to_string();
    let content = tokio::fs::read_to_string(&state.config_path)
        .await
        .unwrap_or_else(|_| "# Config file not found or not yet created".into());
    Ok(Json(
        serde_json::json!({ "path": path, "content": content }),
    ))
}

/// GET /setup/status — return setup completeness for onboarding.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/setup/status", tag = "Config",
    security(()),
    responses((status = 200, description = "Setup completeness status", body = Object))
))]
pub async fn setup_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cfg = state.config.load();
    let mut missing = Vec::new();
    if cfg.user_location.is_none() {
        missing.push("user_location");
    }
    if cfg.user_timezone.is_none() {
        missing.push("user_timezone");
    }
    Json(serde_json::json!({
        "needs_setup": !missing.is_empty(),
        "missing": missing
    }))
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
            gateway_cors_origins: base_state.config.load().gateway_cors_origins.clone(),
            ..Default::default()
        };
        config.gateway_auth_token = Some("super_secret_token".into());
        let state = Arc::new(AppState {
            config: Arc::new(arc_swap::ArcSwap::from_pointee(config)),
            config_path: base_state.config_path.clone(),
            config_write_lock: tokio::sync::Mutex::new(()),
            db: base_state.db.clone(),
            event_bus: base_state.event_bus.clone(),
            memory: base_state.memory.clone(),
            credentials: base_state.credentials.clone(),
            security: base_state.security.clone(),
            tools: Arc::new(crate::tools::ToolRegistry::new()),
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
            embedding_model_available: base_state.embedding_model_available.clone(),
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

    // 8.12.18 — PUT /config with notification_routing persists and returns success
    #[tokio::test]
    async fn update_notification_routing() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("PUT")
            .uri("/config")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "notification_routing": {
                        "scheduler_notification": ["toast", "telegram"],
                        "scheduler_job_completed": ["desktop"],
                        "channel_message": ["toast"]
                    }
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // 8.12.19 — PUT /config with invalid notification_routing is rejected
    #[tokio::test]
    async fn update_notification_routing_invalid() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .method("PUT")
            .uri("/config")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "notification_routing": {
                        "scheduler_notification": ["invalid_target"]
                    }
                }))
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // 8.12.20 — GET /config returns notification_routing field
    #[tokio::test]
    async fn get_config_includes_routing() {
        let (_dir, state) = test_state().await;
        let app = app(state);

        let req = Request::builder()
            .uri("/config")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 16384).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("notification_routing").is_some());
        let routing = &json["notification_routing"];
        assert!(routing.get("scheduler_notification").is_some());
        assert!(routing.get("scheduler_job_completed").is_some());
        assert!(routing.get("channel_message").is_some());
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
        assert_eq!(json["status"], "persisted");
        assert_eq!(json["fields"]["log_level"], "debug");
    }
}
