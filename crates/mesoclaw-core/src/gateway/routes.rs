use std::sync::Arc;

use axum::Router;
use axum::middleware;
use axum::routing::{get, post};
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
        // Providers
        .route("/providers", get(handlers::providers::list_providers))
        .route("/providers/{id}", get(handlers::providers::get_provider))
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
        // WebSocket
        .route("/ws/chat", get(handlers::ws::ws_chat))
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
            .header("origin", "http://localhost:5173")
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
            .header("origin", "http://localhost:5173")
            .header("access-control-request-method", "POST")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
