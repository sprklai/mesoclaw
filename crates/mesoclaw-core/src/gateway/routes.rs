use std::sync::Arc;

use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use super::handlers;
use super::middleware::auth_middleware;
use super::state::AppState;

/// Build the complete axum Router with all routes, middleware, and state.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = build_cors(&state.config.gateway_cors_origins);

    Router::new()
        // System (no auth for health)
        .route("/health", get(handlers::health::health))
        // Sessions
        .route(
            "/sessions",
            post(handlers::sessions::create_session).get(handlers::sessions::list_sessions),
        )
        .route(
            "/sessions/{id}",
            get(handlers::sessions::get_session)
                .put(handlers::sessions::update_session)
                .delete(handlers::sessions::delete_session),
        )
        .route(
            "/sessions/{id}/generate-title",
            post(handlers::sessions::generate_title),
        )
        // Messages
        .route(
            "/sessions/{id}/messages",
            get(handlers::messages::get_messages).post(handlers::messages::send_message),
        )
        // Chat
        .route("/chat", post(handlers::chat::chat))
        // Memory
        .route(
            "/memory",
            post(handlers::memory::create_memory).get(handlers::memory::recall_memories),
        )
        .route(
            "/memory/{key}",
            get(handlers::memory::read_memory_by_key)
                .put(handlers::memory::update_memory)
                .delete(handlers::memory::delete_memory),
        )
        // Config
        .route(
            "/config",
            get(handlers::config::get_config).put(handlers::config::update_config),
        )
        // Credentials (Phase 8)
        .route(
            "/credentials",
            post(handlers::credentials::set_credential)
                .get(handlers::credentials::list_credentials),
        )
        .route(
            "/credentials/{key}",
            delete(handlers::credentials::delete_credential),
        )
        .route(
            "/credentials/{key}/value",
            get(handlers::credentials::get_credential_value),
        )
        .route(
            "/credentials/{key}/exists",
            get(handlers::credentials::credential_exists),
        )
        // Providers (Phase 8 -- multi-provider)
        .route(
            "/providers",
            get(handlers::providers::list_providers)
                .post(handlers::providers::create_user_provider),
        )
        .route(
            "/providers/with-key-status",
            get(handlers::providers::list_with_key_status),
        )
        .route(
            "/providers/default",
            get(handlers::providers::get_default_model).put(handlers::providers::set_default_model),
        )
        .route(
            "/providers/{id}",
            get(handlers::providers::get_provider)
                .put(handlers::providers::update_provider)
                .delete(handlers::providers::delete_user_provider),
        )
        .route(
            "/providers/{id}/test",
            post(handlers::providers::test_connection),
        )
        .route(
            "/providers/{id}/models",
            post(handlers::providers::add_model),
        )
        .route(
            "/providers/{id}/models/{model_id}",
            delete(handlers::providers::delete_model),
        )
        // Tools
        .route("/tools", get(handlers::tools::list_tools))
        .route("/tools/{name}/execute", post(handlers::tools::execute_tool))
        // System info
        .route("/system/info", get(handlers::system::system_info))
        // Models
        .route("/models", get(handlers::models::list_models))
        // Identity (Phase 4)
        .route("/identity", get(handlers::identity::list_identity))
        .route(
            "/identity/reload",
            post(handlers::identity::reload_identity),
        )
        .route(
            "/identity/{name}",
            get(handlers::identity::get_identity_file)
                .put(handlers::identity::update_identity_file),
        )
        // Skills (Phase 4)
        .route(
            "/skills",
            get(handlers::skills::list_skills).post(handlers::skills::create_skill),
        )
        .route("/skills/reload", post(handlers::skills::reload_skills))
        .route(
            "/skills/{id}",
            get(handlers::skills::get_skill)
                .put(handlers::skills::update_skill)
                .delete(handlers::skills::delete_skill),
        )
        // Skill Proposals (Phase 8)
        .route(
            "/skills/proposals",
            get(handlers::skill_proposals::list_proposals),
        )
        .route(
            "/skills/proposals/{id}/approve",
            post(handlers::skill_proposals::approve_proposal),
        )
        .route(
            "/skills/proposals/{id}/reject",
            post(handlers::skill_proposals::reject_proposal),
        )
        .route(
            "/skills/proposals/{id}",
            delete(handlers::skill_proposals::delete_proposal),
        )
        // User (Phase 4)
        .route(
            "/user/observations",
            get(handlers::user::list_observations)
                .post(handlers::user::add_observation)
                .delete(handlers::user::clear_observations),
        )
        .route(
            "/user/observations/{key}",
            get(handlers::user::get_observation_by_key).delete(handlers::user::delete_observation),
        )
        .route("/user/profile", get(handlers::user::get_user_profile))
        // Embeddings (Phase 8.11)
        .route(
            "/embeddings/status",
            get(handlers::embeddings::embeddings_status),
        )
        .route(
            "/embeddings/test",
            post(handlers::embeddings::embeddings_test),
        )
        .route(
            "/embeddings/embed",
            post(handlers::embeddings::embeddings_embed),
        )
        .route(
            "/embeddings/download",
            post(handlers::embeddings::embeddings_download),
        )
        .route(
            "/embeddings/reindex",
            post(handlers::embeddings::embeddings_reindex),
        )
        // Channel credential test (always available, no feature gate)
        .route(
            "/channels/{name}/test",
            post(handlers::channels_test::test_channel_credentials),
        )
        // Channels (Phase 8)
        .merge(channel_routes())
        // Scheduler (Phase 8)
        .merge(scheduler_routes())
        // WebSocket
        .route("/ws/chat", get(handlers::ws::ws_chat))
        .route("/ws/notifications", get(handlers::ws::ws_notifications))
        // Auth middleware
        .layer(middleware::from_fn_with_state(
            state.config.gateway_auth_token.clone(),
            auth_middleware,
        ))
        // CORS
        .layer(cors)
        // Tracing
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Build channel routes, conditionally compiled.
fn channel_routes() -> Router<Arc<AppState>> {
    #[cfg(feature = "channels")]
    {
        Router::new()
            .route(
                "/channels/sessions",
                get(handlers::channels::list_channel_sessions),
            )
            .route(
                "/channels/sessions/{id}/messages",
                get(handlers::channels::list_channel_messages),
            )
            .route("/channels", get(handlers::channels::list_channels))
            .route(
                "/channels/{name}/status",
                get(handlers::channels::channel_status),
            )
            .route(
                "/channels/{name}/send",
                post(handlers::channels::send_message),
            )
            .route(
                "/channels/{name}/connect",
                post(handlers::channels::connect_channel),
            )
            .route(
                "/channels/{name}/disconnect",
                post(handlers::channels::disconnect_channel),
            )
            .route(
                "/channels/{name}/health",
                get(handlers::channels::health_check),
            )
            .route(
                "/channels/{name}/message",
                post(handlers::channels::webhook_message),
            )
    }
    #[cfg(not(feature = "channels"))]
    {
        Router::new()
    }
}

/// Build scheduler routes, conditionally compiled.
fn scheduler_routes() -> Router<Arc<AppState>> {
    #[cfg(feature = "scheduler")]
    {
        use axum::routing::put;
        Router::new()
            .route(
                "/scheduler/jobs",
                get(handlers::scheduler::list_jobs).post(handlers::scheduler::create_job),
            )
            .route(
                "/scheduler/jobs/{id}/toggle",
                put(handlers::scheduler::toggle_job),
            )
            .route(
                "/scheduler/jobs/{id}",
                delete(handlers::scheduler::delete_job),
            )
            .route(
                "/scheduler/jobs/{id}/history",
                get(handlers::scheduler::job_history),
            )
            .route(
                "/scheduler/status",
                get(handlers::scheduler::scheduler_status),
            )
    }
    #[cfg(not(feature = "scheduler"))]
    {
        Router::new()
    }
}

fn build_cors(origins: &[String]) -> CorsLayer {
    if origins.is_empty() || origins.iter().any(|o| o == "*") {
        CorsLayer::permissive()
    } else {
        let origins: Vec<_> = origins.iter().filter_map(|o| o.parse().ok()).collect();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    }
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

    // 4.1.1 — invalid route returns 404
    #[tokio::test]
    async fn invalid_route_returns_404() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/nonexistent")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // 4.1.2 — CORS headers present
    #[tokio::test]
    async fn cors_headers_present() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .uri("/health")
            .header("origin", "http://localhost:18971")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get("access-control-allow-origin").is_some());
    }

    // 4.1.3 — OPTIONS preflight returns 200
    #[tokio::test]
    async fn options_preflight_returns_200() {
        let (_dir, state) = test_state().await;
        let app = build_router(state);

        let req = Request::builder()
            .method("OPTIONS")
            .uri("/sessions")
            .header("origin", "http://localhost:18971")
            .header("access-control-request-method", "POST")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
