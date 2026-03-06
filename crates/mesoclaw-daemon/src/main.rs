use std::path::PathBuf;

use clap::Parser;
use tracing::{error, info};

use mesoclaw_core::config::{default_config_path, default_data_dir, load_or_create_config};
use mesoclaw_core::db;

#[derive(Parser)]
#[command(name = "mesoclaw-daemon", about = "MesoClaw headless daemon")]
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
            // Tracing isn't initialized yet, so use eprintln for early messages
            eprintln!("Config loaded from {}", config_path.display());
            c
        }
        Err(e) => {
            eprintln!("Failed to load config from {}: {e}", config_path.display());
            std::process::exit(1);
        }
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.parse().unwrap_or_default()),
        )
        .init();

    info!(identity = %config.identity_name, "Starting MesoClaw daemon");

    let data_dir = config
        .data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(default_data_dir);

    let db_path = config
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("mesoclaw.db"));

    let pool = match db::init_pool(&db_path) {
        Ok(p) => {
            info!(path = %db_path.display(), "Database initialized");
            p
        }
        Err(e) => {
            error!(
                "Failed to initialize database at {}: {e}",
                db_path.display()
            );
            std::process::exit(1);
        }
    };

    if let Err(e) = db::with_db(&pool, db::run_migrations).await {
        error!("Failed to run migrations: {e}");
        std::process::exit(1);
    }
    info!("Database migrations applied");

    info!(
        host = %config.gateway_host,
        port = %config.gateway_port,
        "MesoClaw daemon ready"
    );

    // TODO: Start axum gateway server here
    // For now, wait for shutdown signal
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    info!("Shutting down");
}
