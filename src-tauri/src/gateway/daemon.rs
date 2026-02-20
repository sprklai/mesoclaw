use std::{fs, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use tower_http::set_header::SetResponseHeaderLayer;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::{
    agent::{agent_commands::SessionCancelMap, session_router::SessionRouter},
    database::DbPool,
    event_bus::EventBus,
    identity::IdentityLoader,
    memory::store::InMemoryStore,
    modules::ModuleRegistry,
    scheduler::TokioScheduler,
};

use super::{
    auth::{auth_middleware, load_or_create_token},
    routes::{
        GatewayState, create_scheduler_job, create_session, delete_scheduler_job, forget_memory,
        get_identity_file, health, list_identity_files, list_memory, list_modules,
        list_scheduler_jobs, list_sessions, module_health, provider_status, reload_modules,
        scheduler_job_history, search_memory, send_approval, start_module, stop_module,
        store_memory, toggle_scheduler_job, update_identity_file,
    },
    ws::ws_handler,
};

const DEFAULT_PORT: u16 = 18790;
const MAX_PORT_ATTEMPTS: u16 = 10;

/// Path to the PID file written on daemon start.
pub fn pid_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".mesoclaw")
        .join("daemon.pid")
}

/// Start the HTTP gateway daemon, binding to `127.0.0.1` starting at port
/// [`DEFAULT_PORT`].  If that port is taken, increments up to
/// [`MAX_PORT_ATTEMPTS`] times before returning an error.
///
/// Writes `daemon.pid` on successful bind.  Blocks until the server shuts down.
pub async fn start_gateway(
    bus: Arc<dyn EventBus>,
    sessions: Arc<SessionRouter>,
    modules: Arc<ModuleRegistry>,
    db_pool: DbPool,
    identity_loader: Arc<IdentityLoader>,
    memory: Arc<InMemoryStore>,
    scheduler: Arc<TokioScheduler>,
    cancel_map: SessionCancelMap,
) -> Result<(), String> {
    // Ensure the token exists before accepting connections.
    load_or_create_token()?;

    let state = GatewayState {
        bus,
        sessions,
        modules,
        db_pool,
        identity_loader,
        memory,
        scheduler,
        cancel_map,
    };

    // Build the router.
    let protected = Router::new()
        .route("/api/v1/sessions", post(create_session).get(list_sessions))
        .route("/api/v1/providers", get(provider_status))
        .route("/api/v1/ws", get(ws_handler))
        // Module management endpoints (Phase 5.5)
        .route("/api/v1/modules", get(list_modules).post(reload_modules))
        .route("/api/v1/modules/{id}/health", get(module_health))
        .route("/api/v1/modules/{id}/start", post(start_module))
        .route("/api/v1/modules/{id}/stop", post(stop_module))
        // Identity endpoints
        .route("/api/v1/identity", get(list_identity_files))
        .route(
            "/api/v1/identity/{file}",
            get(get_identity_file).put(update_identity_file),
        )
        // Memory endpoints
        .route("/api/v1/memory", get(list_memory).post(store_memory))
        .route("/api/v1/memory/search", get(search_memory))
        .route("/api/v1/memory/{key}", delete(forget_memory))
        // Approval endpoint (used by CLI and any headless client)
        .route("/api/v1/approval/{action_id}", post(send_approval))
        // Scheduler endpoints
        .route(
            "/api/v1/scheduler/jobs",
            get(list_scheduler_jobs).post(create_scheduler_job),
        )
        .route(
            "/api/v1/scheduler/jobs/{job_id}/toggle",
            axum::routing::put(toggle_scheduler_job),
        )
        .route(
            "/api/v1/scheduler/jobs/{job_id}",
            delete(delete_scheduler_job),
        )
        .route(
            "/api/v1/scheduler/jobs/{job_id}/history",
            get(scheduler_job_history),
        )
        .layer(middleware::from_fn(auth_middleware))
        .with_state(state.clone());

    let public = Router::new().route("/api/v1/health", get(health));

    let app = Router::new()
        .merge(public)
        .merge(protected)
        .layer(CorsLayer::permissive()) // Restrict to localhost in production (Phase 2.8+)
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::HeaderName::from_static("x-api-version"),
            axum::http::header::HeaderValue::from_static("v1"),
        ));

    // Try ports starting at DEFAULT_PORT.
    let listener = bind_with_fallback(DEFAULT_PORT).await?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;

    write_pid_file(addr.port())?;
    log::info!("Mesoclaw daemon listening on {addr}");

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("daemon error: {e}"))
}

async fn bind_with_fallback(start_port: u16) -> Result<TcpListener, String> {
    for offset in 0..MAX_PORT_ATTEMPTS {
        let port = start_port + offset;
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("valid addr");
        match TcpListener::bind(addr).await {
            Ok(listener) => return Ok(listener),
            Err(_) if offset + 1 < MAX_PORT_ATTEMPTS => continue,
            Err(e) => {
                return Err(format!(
                    "could not bind to any port in {start_port}–{}: {e}",
                    start_port + MAX_PORT_ATTEMPTS - 1
                ));
            }
        }
    }
    unreachable!()
}

fn write_pid_file(port: u16) -> Result<(), String> {
    let path = pid_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create .mesoclaw dir: {e}"))?;
    }
    let content = format!("{}\n{}\n", std::process::id(), port);
    fs::write(&path, content).map_err(|e| format!("failed to write PID file: {e}"))
}
