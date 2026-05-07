pub mod errors;
pub mod handlers;
pub mod middleware;
#[cfg(feature = "api-docs")]
pub mod openapi;
pub mod routes;
pub mod state;

use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::{Result, ZeniiError};
use state::AppState;

const WIKI_MAINTENANCE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// The gateway HTTP+WS server.
pub struct GatewayServer {
    state: Arc<AppState>,
}

impl GatewayServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Start the gateway server with a shutdown signal.
    ///
    /// If `ready_tx` is provided, a message is sent once the TCP listener is bound
    /// and the server is ready to accept connections.
    pub async fn start_with_shutdown(
        self,
        host: &str,
        port: u16,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
        ready_tx: Option<tokio::sync::oneshot::Sender<()>>,
    ) -> Result<()> {
        let router = routes::build_router(Arc::clone(&self.state));
        let addr = format!("{host}:{port}");

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ZeniiError::Gateway(format!("failed to bind to {addr}: {e}")))?;

        info!("Gateway listening on {addr}");

        if let Some(tx) = ready_tx {
            let _ = tx.send(());
        }

        tokio::spawn(wiki_maintenance_loop(Arc::clone(&self.state)));

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|e| ZeniiError::Gateway(format!("server error: {e}")))?;

        info!("Gateway shut down cleanly");
        Ok(())
    }
}

/// Runs lint auto-fix every [`WIKI_MAINTENANCE_INTERVAL`].
///
/// Only applies safe, non-destructive fixes (e.g. backfilling missing `updated`
/// dates). Stub deletion and source-ref cleanup are intentionally excluded —
/// those require explicit user action.
async fn wiki_maintenance_loop(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(WIKI_MAINTENANCE_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await; // skip the immediate first tick on startup

    loop {
        interval.tick().await;
        info!("Wiki maintenance: running lint auto-fix");

        let wiki = Arc::clone(&state.wiki);
        let result = tokio::task::spawn_blocking(move || {
            let wiki = wiki.blocking_lock();
            let issues = wiki.lint()?;
            let (fixed, remaining) = wiki.lint_fix(&issues)?;
            Ok::<_, ZeniiError>((remaining.len(), fixed.len()))
        })
        .await;

        match result {
            Ok(Ok((remaining, fixed))) => {
                info!(
                    "Wiki maintenance done: {fixed} auto-fixed, {remaining} issues need manual review"
                )
            }
            Ok(Err(e)) => warn!("Wiki maintenance error: {e}"),
            Err(e) => warn!("Wiki maintenance task panicked: {e}"),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "ai")]
mod tests {
    use super::*;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    /// Find a free port by binding to port 0, extracting the assigned port, then dropping the listener.
    async fn free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    // 4.3.1 — server binds to port and serves health endpoint
    #[tokio::test]
    async fn server_binds_to_port() {
        let (_dir, state) = test_state().await;
        let port = free_port().await;

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = GatewayServer::new(state);
        let handle = tokio::spawn(async move {
            server
                .start_with_shutdown(
                    "127.0.0.1",
                    port,
                    async {
                        let _ = rx.await;
                    },
                    None,
                )
                .await
        });

        // Give the server a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let url = format!("http://127.0.0.1:{port}/health");
        let resp = reqwest::get(&url).await.expect("failed to reach server");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        // Shut down
        let _ = tx.send(());
        let result = handle.await.expect("server task panicked");
        assert!(result.is_ok());
    }

    // 4.3.2 — server shuts down gracefully on signal
    #[tokio::test]
    async fn server_shutdown_graceful() {
        let (_dir, state) = test_state().await;
        let port = free_port().await;

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = GatewayServer::new(state);
        let handle = tokio::spawn(async move {
            server
                .start_with_shutdown(
                    "127.0.0.1",
                    port,
                    async {
                        let _ = rx.await;
                    },
                    None,
                )
                .await
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Confirm it's running
        let url = format!("http://127.0.0.1:{port}/health");
        let resp = reqwest::get(&url).await.expect("failed to reach server");
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        // Send shutdown signal
        let _ = tx.send(());

        // Server task should complete without error
        let result = handle.await.expect("server task panicked");
        assert!(
            result.is_ok(),
            "server did not shut down cleanly: {result:?}"
        );
    }
}
