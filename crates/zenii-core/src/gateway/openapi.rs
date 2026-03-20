use std::sync::Arc;

use axum::Router;
use utoipa::OpenApi;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa_scalar::{Scalar, Servable};

use super::handlers;
use super::state::AppState;

/// Base API spec — all non-feature-gated endpoints.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Zenii API",
        version = "0.0.33",
        description = "Zenii AI assistant gateway API.\n\nAuthentication: Bearer token via `Authorization: Bearer <token>` header.\nWebSocket endpoints also accept `?token=<token>` query parameter.",
        contact(name = "SprklAI by NSRTech", url = "https://zenii.sprklai.com"),
        license(name = "MIT", url = "https://github.com/sprklai/zenii/blob/main/LICENSE"),
    ),
    paths(
        // System
        handlers::health::health,
        handlers::system::system_info,
        // Sessions
        handlers::sessions::create_session,
        handlers::sessions::list_sessions,
        handlers::sessions::get_session,
        handlers::sessions::update_session,
        handlers::sessions::delete_session,
        handlers::sessions::generate_title,
        // Messages
        handlers::messages::get_messages,
        handlers::messages::send_message,
        // Chat
        handlers::chat::chat,
        // Memory
        handlers::memory::create_memory,
        handlers::memory::recall_memories,
        handlers::memory::read_memory_by_key,
        handlers::memory::update_memory,
        handlers::memory::delete_memory,
        // Config
        handlers::config::get_config,
        handlers::config::update_config,
        handlers::config::get_config_file,
        handlers::config::setup_status,
        // Credentials
        handlers::credentials::set_credential,
        handlers::credentials::list_credentials,
        handlers::credentials::delete_credential,
        handlers::credentials::get_credential_value,
        handlers::credentials::credential_exists,
        // Providers
        handlers::providers::list_providers,
        handlers::providers::list_with_key_status,
        handlers::providers::get_default_model,
        handlers::providers::set_default_model,
        handlers::providers::get_provider,
        handlers::providers::create_user_provider,
        handlers::providers::update_provider,
        handlers::providers::delete_user_provider,
        handlers::providers::add_model,
        handlers::providers::test_connection,
        handlers::providers::delete_model,
        // Tools
        handlers::tools::list_tools,
        handlers::tools::execute_tool,
        // Models
        handlers::models::list_models,
        // Identity
        handlers::identity::list_identity,
        handlers::identity::get_identity_file,
        handlers::identity::update_identity_file,
        handlers::identity::reload_identity,
        // Skills
        handlers::skills::list_skills,
        handlers::skills::get_skill,
        handlers::skills::create_skill,
        handlers::skills::update_skill,
        handlers::skills::delete_skill,
        handlers::skills::reload_skills,
        // Skill Proposals
        handlers::skill_proposals::list_proposals,
        handlers::skill_proposals::approve_proposal,
        handlers::skill_proposals::reject_proposal,
        handlers::skill_proposals::delete_proposal,
        // User
        handlers::user::list_observations,
        handlers::user::add_observation,
        handlers::user::get_observation_by_key,
        handlers::user::delete_observation,
        handlers::user::clear_observations,
        handlers::user::get_user_profile,
        // Embeddings
        handlers::embeddings::embeddings_status,
        handlers::embeddings::embeddings_test,
        handlers::embeddings::embeddings_embed,
        handlers::embeddings::embeddings_download,
        handlers::embeddings::embeddings_reindex,
        // Plugins
        handlers::plugins::list_plugins,
        handlers::plugins::get_plugin,
        handlers::plugins::install_plugin,
        handlers::plugins::remove_plugin,
        handlers::plugins::toggle_plugin,
        handlers::plugins::update_plugin,
        handlers::plugins::get_plugin_config,
        handlers::plugins::update_plugin_config,
        // Channel credential test (always available)
        handlers::channels_test::test_channel_credentials,
        // WebSocket
        handlers::ws::ws_chat,
        handlers::ws::ws_notifications,
    ),
    components(
        schemas(
            super::errors::ErrorResponse,
            handlers::sessions::CreateSessionRequest,
            handlers::sessions::UpdateSessionRequest,
            handlers::sessions::GenerateTitleRequest,
            handlers::messages::SendMessageRequest,
            handlers::messages::MessageWithToolCalls,
            handlers::chat::ChatRequest,
            handlers::chat::ChatResponse,
            handlers::memory::StoreMemoryRequest,
            handlers::memory::UpdateMemoryRequest,
            handlers::memory::RecallQuery,
            handlers::credentials::SetCredentialRequest,
            handlers::credentials::CredentialExistsResponse,
            handlers::providers::CreateProviderRequest,
            handlers::providers::CreateModelEntry,
            handlers::providers::UpdateProviderRequest,
            handlers::providers::AddModelRequest,
            handlers::providers::SetDefaultModelRequest,
            handlers::tools::ExecuteToolRequest,
            handlers::identity::IdentityListResponse,
            handlers::identity::IdentityFileInfo,
            handlers::identity::IdentityFileResponse,
            handlers::identity::UpdateIdentityRequest,
            handlers::skills::SkillsListResponse,
            handlers::skills::CreateSkillRequest,
            handlers::skill_proposals::SkillProposal,
            handlers::user::ObservationsListResponse,
            handlers::user::AddObservationRequest,
            handlers::user::UserProfileResponse,
            handlers::embeddings::EmbeddingStatus,
            handlers::embeddings::EmbedRequest,
            handlers::embeddings::EmbedTestResult,
            handlers::plugins::PluginListItem,
            handlers::plugins::InstallRequest,
            handlers::channels_test::ChannelTestResult,
        )
    ),
    security(("bearer" = [])),
)]
struct BaseApiDoc;

/// Channel API spec (feature-gated).
#[cfg(feature = "channels")]
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::channels::list_channels,
        handlers::channels::channel_status,
        handlers::channels::send_message,
        handlers::channels::connect_channel,
        handlers::channels::disconnect_channel,
        handlers::channels::list_channel_sessions,
        handlers::channels::list_channel_messages,
        handlers::channels::webhook_message,
        handlers::channels::health_check,
    ),
    components(schemas(
        handlers::channels::ChannelInfo,
        handlers::channels::ChannelHealthResponse,
        handlers::channels::SendMessageRequest,
    ))
)]
struct ChannelsApiDoc;

/// Scheduler API spec (feature-gated).
#[cfg(feature = "scheduler")]
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::scheduler::list_jobs,
        handlers::scheduler::create_job,
        handlers::scheduler::toggle_job,
        handlers::scheduler::delete_job,
        handlers::scheduler::job_history,
        handlers::scheduler::scheduler_status,
    ),
    components(schemas(
        handlers::scheduler::SchedulerStatusResponse,
        handlers::scheduler::CreateJobResponse,
        handlers::scheduler::ToggleResponse,
    ))
)]
struct SchedulerApiDoc;

/// Build the merged OpenAPI spec.
pub fn build_openapi() -> utoipa::openapi::OpenApi {
    let mut spec = BaseApiDoc::openapi();

    // Add security scheme definition
    let components = spec.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
        "bearer",
        SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
    );

    #[cfg(feature = "channels")]
    spec.merge(ChannelsApiDoc::openapi());

    #[cfg(feature = "scheduler")]
    spec.merge(SchedulerApiDoc::openapi());

    spec
}

/// Build the router serving Scalar UI + OpenAPI JSON.
pub fn openapi_routes() -> Router<Arc<AppState>> {
    let spec = build_openapi();
    let spec_json = serde_json::to_string_pretty(&spec).unwrap_or_default();

    Router::new()
        .merge(Scalar::with_url("/api-docs", spec))
        .route(
            "/api-docs/openapi.json",
            axum::routing::get(move || async move {
                (
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    spec_json,
                )
            }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_spec_generates_valid_json() {
        let spec = build_openapi();
        let json = serde_json::to_string_pretty(&spec).expect("spec should serialize to JSON");
        assert!(json.contains("\"openapi\""));
        assert!(json.contains("Zenii API"));
    }

    #[test]
    fn openapi_spec_has_expected_paths() {
        let spec = build_openapi();
        let paths = spec.paths;
        // Check some key paths exist
        assert!(paths.paths.contains_key("/health"));
        assert!(paths.paths.contains_key("/sessions"));
        assert!(paths.paths.contains_key("/chat"));
        assert!(paths.paths.contains_key("/memory"));
        assert!(paths.paths.contains_key("/config"));
        assert!(paths.paths.contains_key("/credentials"));
        assert!(paths.paths.contains_key("/providers"));
        assert!(paths.paths.contains_key("/tools"));
        assert!(paths.paths.contains_key("/identity"));
        assert!(paths.paths.contains_key("/skills"));
        assert!(paths.paths.contains_key("/plugins"));
    }

    #[test]
    fn openapi_spec_has_security_scheme() {
        let spec = build_openapi();
        let components = spec.components.expect("should have components");
        assert!(components.security_schemes.contains_key("bearer"));
    }

    #[tokio::test]
    async fn openapi_routes_serve_spec() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let (_, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        // Test /api-docs/openapi.json
        let req = Request::builder()
            .uri("/api-docs/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn scalar_ui_serves_html() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let (_, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);

        let req = Request::builder()
            .uri("/api-docs")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_docs_bypasses_auth() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        // Create state with auth enabled
        let (dir, base_state) = crate::gateway::handlers::tests::test_state().await;
        let mut config = crate::config::AppConfig {
            gateway_cors_origins: base_state.config.load().gateway_cors_origins.clone(),
            ..Default::default()
        };
        config.gateway_auth_token = Some("secret".into());

        let state = std::sync::Arc::new(crate::gateway::state::AppState {
            config: std::sync::Arc::new(arc_swap::ArcSwap::from_pointee(config)),
            config_path: dir.path().join("config.toml"),
            config_write_lock: tokio::sync::Mutex::new(()),
            db: base_state.db.clone(),
            event_bus: base_state.event_bus.clone(),
            memory: base_state.memory.clone(),
            credentials: base_state.credentials.clone(),
            security: base_state.security.clone(),
            tools: base_state.tools.clone(),
            session_manager: base_state.session_manager.clone(),
            agent: None,
            provider_registry: base_state.provider_registry.clone(),
            boot_context: base_state.boot_context.clone(),
            last_used_model: base_state.last_used_model.clone(),
            context_builder: base_state.context_builder.clone(),
            reasoning_engine: base_state.reasoning_engine.clone(),
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
            coordinator: base_state.coordinator.clone(),
            #[cfg(feature = "workflows")]
            workflow_registry: None,
            #[cfg(feature = "workflows")]
            workflow_executor: None,
            #[cfg(feature = "workflows")]
            active_workflow_runs: Arc::new(dashmap::DashMap::new()),
            usage_logger: base_state.usage_logger.clone(),
            embedding_model_available: base_state.embedding_model_available.clone(),
        });

        let app = crate::gateway::routes::build_router(state);

        // api-docs should work without auth
        let req = Request::builder()
            .uri("/api-docs/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
