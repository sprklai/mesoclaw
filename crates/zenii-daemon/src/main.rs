use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing::{error, info};

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
async fn main() {
    let args = Args::parse();

    let config_path = args.config.unwrap_or_else(default_config_path);

    let config = match load_or_create_config(&config_path) {
        Ok(c) => {
            eprintln!("Config loaded from {}", config_path.display());
            c
        }
        Err(e) => {
            eprintln!("Failed to load config from {}: {e}", config_path.display());
            std::process::exit(1);
        }
    };

    if let Err(e) = zenii_core::logging::init_tracing(&config, "daemon", false) {
        eprintln!("Failed to initialize tracing: {e}");
        std::process::exit(1);
    }

    info!(identity = %config.identity_name, "Starting Zenii daemon");

    let host = config.gateway_host.clone();
    let port = config.gateway_port;

    // Initialize all services
    let services = match boot::init_services(config).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize services: {e}");
            std::process::exit(1);
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
            tokio::signal::ctrl_c().await.ok();
        }
        info!("Shutdown signal received, draining connections...");
    };

    if let Err(e) = gateway
        .start_with_shutdown(&host, port, shutdown, None)
        .await
    {
        error!("Gateway server error: {e}");
        std::process::exit(1);
    }
}
