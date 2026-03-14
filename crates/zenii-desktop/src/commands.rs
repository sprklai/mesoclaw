use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use tokio::sync::oneshot;
use tracing::info;

/// Holds the gateway shutdown sender so we can stop it when the app exits.
pub struct GatewayState {
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub external_url: Option<String>,
}

/// Configuration for the gateway boot decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMode {
    pub external_url: Option<String>,
}

/// Determine the gateway mode from the environment.
///
/// Returns `Some(url)` if `ZENII_GATEWAY_URL` is set and valid,
/// `None` if the embedded gateway should be started.
pub fn resolve_gateway_mode() -> Result<GatewayMode, String> {
    match std::env::var("ZENII_GATEWAY_URL") {
        Ok(url_str) if !url_str.is_empty() => {
            // Validate the URL
            url::Url::parse(&url_str)
                .map_err(|e| format!("Invalid ZENII_GATEWAY_URL '{url_str}': {e}"))?;
            info!("Using external gateway at {url_str}");
            Ok(GatewayMode {
                external_url: Some(url_str),
            })
        }
        _ => {
            info!("No external gateway configured, will start embedded");
            Ok(GatewayMode { external_url: None })
        }
    }
}

/// Resolve the Zenii data directory path.
pub fn resolve_data_dir() -> std::path::PathBuf {
    zenii_core::config::default_data_dir()
}

/// Boot the embedded gateway server in a background task.
///
/// This is called from the Tauri `.setup()` hook when no external URL is configured.
#[allow(clippy::unwrap_used)]
pub fn boot_gateway(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing so gateway logs are visible in the terminal.
    // Use try_init() because Tauri devtools may have already set a global subscriber.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "zenii_core=info,warn".parse().unwrap()),
        )
        .try_init();

    let mode = resolve_gateway_mode().map_err(|e| e.to_string())?;

    if mode.external_url.is_some() {
        // External gateway — just store the state, no embedded boot needed
        app.manage(Arc::new(tokio::sync::Mutex::new(GatewayState {
            shutdown_tx: None,
            external_url: mode.external_url,
        })));
        return Ok(());
    }

    // Load config
    let config_path = zenii_core::config::default_config_path();
    let config = zenii_core::config::load_or_create_config(&config_path)?;

    let host = config.gateway_host.clone();
    let port = config.gateway_port;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Spawn the gateway in a background task
    tauri::async_runtime::spawn(async move {
        match zenii_core::boot::init_services(config).await {
            Ok(services) => {
                let state = Arc::new(zenii_core::gateway::state::AppState::from(services));
                #[cfg(feature = "scheduler")]
                state.wire_scheduler();
                #[cfg(feature = "channels")]
                state.wire_channels();
                state.wire_notifications();
                let gateway = zenii_core::gateway::GatewayServer::new(state);

                info!("Starting embedded gateway on {host}:{port}");
                if let Err(e) = gateway
                    .start_with_shutdown(&host, port, async {
                        let _ = shutdown_rx.await;
                    })
                    .await
                {
                    tracing::error!("Embedded gateway error: {e}");
                }
            }
            Err(e) => {
                tracing::error!("Failed to initialize services: {e}");
            }
        }
    });

    app.manage(Arc::new(tokio::sync::Mutex::new(GatewayState {
        shutdown_tx: Some(shutdown_tx),
        external_url: None,
    })));

    Ok(())
}

// --- IPC Commands ---

#[tauri::command]
pub fn show_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn show_notification(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_data_dir() -> Result<(), String> {
    let data_dir = resolve_data_dir();
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    opener::open(data_dir.to_string_lossy().as_ref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_config_file() -> Result<String, String> {
    let config_path = zenii_core::config::default_config_path();
    let backup_path = config_path.with_extension("toml.bak");
    if config_path.exists() {
        std::fs::copy(&config_path, &backup_path).map_err(|e| e.to_string())?;
    }
    opener::open(config_path.to_string_lossy().as_ref()).map_err(|e| e.to_string())?;
    Ok(backup_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mutex to serialize tests that manipulate ZENII_GATEWAY_URL env var
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // 7.1 — External gateway URL skips embedded boot
    #[test]
    fn external_gateway_skips_embedded() {
        let _guard = ENV_LOCK.lock();
        // SAFETY: test-only env manipulation, serialized by ENV_LOCK
        unsafe {
            std::env::set_var("ZENII_GATEWAY_URL", "http://localhost:9999");
        }
        let mode = resolve_gateway_mode().unwrap();
        assert!(mode.external_url.is_some());
        assert_eq!(mode.external_url.unwrap(), "http://localhost:9999");
        unsafe {
            std::env::remove_var("ZENII_GATEWAY_URL");
        }
    }

    // 7.2 — Invalid external URL returns error
    #[test]
    fn invalid_external_url_returns_error() {
        let _guard = ENV_LOCK.lock();
        unsafe {
            std::env::set_var("ZENII_GATEWAY_URL", "not a valid url");
        }
        let result = resolve_gateway_mode();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid ZENII_GATEWAY_URL"));
        unsafe {
            std::env::remove_var("ZENII_GATEWAY_URL");
        }
    }

    // 7.3 — Resolve data dir returns valid path
    #[test]
    fn resolve_data_dir_returns_valid_path() {
        let dir = resolve_data_dir();
        // Should return a non-empty path
        assert!(!dir.as_os_str().is_empty());
    }

    // 7.4 — Resolve data dir uses app identifier
    #[test]
    fn resolve_data_dir_uses_app_id() {
        let dir = resolve_data_dir();
        let path_str = dir.to_string_lossy();
        // The directories crate uses the qualifier/org/app from project_dirs
        // which includes "zenii" in the path
        assert!(
            path_str.contains("zenii") || path_str.contains("sprklai"),
            "Data dir path should contain app identifier, got: {path_str}"
        );
    }

    // 7.1b — No env var means embedded mode
    #[test]
    fn no_env_var_means_embedded_mode() {
        let _guard = ENV_LOCK.lock();
        unsafe {
            std::env::remove_var("ZENII_GATEWAY_URL");
        }
        let mode = resolve_gateway_mode().unwrap();
        assert!(mode.external_url.is_none());
    }

    // 7.1c — Empty env var means embedded mode
    #[test]
    fn empty_env_var_means_embedded_mode() {
        let _guard = ENV_LOCK.lock();
        unsafe {
            std::env::set_var("ZENII_GATEWAY_URL", "");
        }
        let mode = resolve_gateway_mode().unwrap();
        assert!(mode.external_url.is_none());
        unsafe {
            std::env::remove_var("ZENII_GATEWAY_URL");
        }
    }
}
