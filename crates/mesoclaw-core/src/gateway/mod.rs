pub mod errors;
pub mod handlers;
pub mod middleware;
#[cfg(feature = "api-docs")]
pub mod openapi;
pub mod routes;
pub mod state;

use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::info;

use crate::{MesoError, Result};
use state::AppState;

/// The gateway HTTP+WS server.
pub struct GatewayServer {
    state: Arc<AppState>,
}

impl GatewayServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Start the gateway server with a shutdown signal.
    pub async fn start_with_shutdown(
        self,
        host: &str,
        port: u16,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<()> {
        let router = routes::build_router(self.state);
        let addr = format!("{host}:{port}");

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| MesoError::Gateway(format!("failed to bind to {addr}: {e}")))?;

        info!("Gateway listening on {addr}");

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|e| MesoError::Gateway(format!("server error: {e}")))?;

        info!("Gateway shut down cleanly");
        Ok(())
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
                .start_with_shutdown("127.0.0.1", port, async {
                    let _ = rx.await;
                })
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
                .start_with_shutdown("127.0.0.1", port, async {
                    let _ = rx.await;
                })
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
