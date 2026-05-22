use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use tracing::{error, info, warn};

use zenii_core::boot;
use zenii_core::config::{default_config_path, load_or_create_config};
use zenii_core::gateway::GatewayServer;

#[derive(Parser)]
#[command(name = "zenii-daemon", about = "Zenii headless daemon")]
struct Args {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();

    let config_path = args.config.unwrap_or_else(default_config_path);

    let config = match load_or_create_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config from {}: {e}", config_path.display());
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = zenii_core::logging::init_tracing(&config, "daemon", false) {
        eprintln!("Failed to initialize tracing: {e}");
        return ExitCode::FAILURE;
    }

    info!("Config loaded from {}", config_path.display());
    info!(identity = %config.identity_name, "Starting Zenii daemon");

    let host = config.gateway_host.clone();
    let port = config.gateway_port;

    if !config.allow_remote_binding && !is_loopback(&host) {
        error!(
            host = %host,
            "gateway_host is not a loopback address; set allow_remote_binding = true in config to permit this"
        );
        return ExitCode::FAILURE;
    }
    if config.allow_remote_binding && !is_loopback(&host) {
        warn!(host = %host, "Binding gateway to non-loopback address — API is reachable from the network");
    }

    // Initialize all services
    let services = match boot::init_services(config).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize services: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Convert services into gateway AppState
    let state = Arc::new(zenii_core::gateway::state::AppState::from(services));
    #[cfg(feature = "scheduler")]
    state.wire_scheduler();
    #[cfg(feature = "channels")]
    state.wire_channels();
    state.wire_notifications();
    let gateway = GatewayServer::new(state);

    // Graceful shutdown on SIGTERM/SIGINT
    let shutdown = async {
        #[cfg(unix)]
        {
            let Ok(mut sigterm) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            else {
                error!("Failed to register SIGTERM handler, falling back to SIGINT only");
                tokio::signal::ctrl_c().await.ok();
                return;
            };
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = sigterm.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!("Failed to register Ctrl-C handler: {e}");
            }
        }
        info!("Shutdown signal received, draining connections...");
    };

    if let Err(e) = gateway
        .start_with_shutdown(&host, port, shutdown, None)
        .await
    {
        error!("Gateway server error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn is_loopback(host: &str) -> bool {
    host == "127.0.0.1" || host == "::1" || host == "localhost"
}
