use std::path::PathBuf;

use clap::Parser;
use rmcp::ServiceExt;
use tracing::{error, info};

use zenii_core::config::{default_config_path, load_or_create_config};
use zenii_core::mcp::ZeniiMcpServer;

#[derive(Parser)]
#[command(
    name = "zenii-mcp-server",
    about = "Zenii MCP server — expose tools via Model Context Protocol"
)]
struct Args {
    /// Path to config.toml
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Transport mode (only "stdio" supported currently)
    #[arg(short, long, default_value = "stdio")]
    transport: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);

    let config = match load_or_create_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            std::process::exit(1);
        }
    };

    // Tracing must go to stderr — stdout is reserved for MCP JSON-RPC protocol
    zenii_core::logging::init_tracing(&config, "mcp-server", false)
        .unwrap_or_else(|e| eprintln!("Failed to init tracing: {e}"));

    let services = match zenii_core::boot::init_services(config).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to initialize services: {e}");
            std::process::exit(1);
        }
    };

    let handler = ZeniiMcpServer::new(
        services.tools.clone(),
        services.security.clone(),
        services.config_swap.clone(),
    );

    info!(
        tool_count = services.tools.len(),
        transport = %args.transport,
        "Starting Zenii MCP server"
    );

    match args.transport.as_str() {
        "stdio" => {
            let service = handler
                .serve(rmcp::transport::io::stdio())
                .await
                .unwrap_or_else(|e| {
                    error!("MCP server initialization failed: {e}");
                    std::process::exit(1);
                });
            // Block until client disconnects
            let _ = service.waiting().await;
        }
        other => {
            error!(
                transport = other,
                "Unsupported transport. Only 'stdio' is supported."
            );
            std::process::exit(1);
        }
    }
}
