use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};

use axum::{
    Router,
    middleware,
    routing::{get, post},
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::event_bus::EventBus;

use super::{
    auth::{auth_middleware, load_or_create_token},
    routes::{
        GatewayState,
        create_session, health, list_sessions, provider_status,
        list_modules, module_health, start_module, stop_module, reload_modules,
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
pub async fn start_gateway(bus: Arc<dyn EventBus>) -> Result<(), String> {
    // Ensure the token exists before accepting connections.
    load_or_create_token()?;

    let state: GatewayState = bus;

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
        .layer(middleware::from_fn(auth_middleware))
        .with_state(state.clone());

    let public = Router::new().route("/api/v1/health", get(health));

    let app = Router::new()
        .merge(public)
        .merge(protected)
        .layer(CorsLayer::permissive()); // Restrict to localhost in production (Phase 2.8+)

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
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create .mesoclaw dir: {e}"))?;
    }
    let content = format!("{}\n{}\n", std::process::id(), port);
    fs::write(&path, content)
        .map_err(|e| format!("failed to write PID file: {e}"))
}
