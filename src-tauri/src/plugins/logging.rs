use std::{path::PathBuf, sync::OnceLock};

// Keeps the non-blocking writer alive for the lifetime of the process.
static GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// Initialise the tracing subscriber with a rolling daily log file.
///
/// Log directory (platform-specific):
///   macOS   → ~/Library/Logs/com.sprklai.mesoclaw/
///   Linux   → ~/.local/share/com.sprklai.mesoclaw/
///   Windows → %APPDATA%\com.sprklai.mesoclaw\
///
/// Verbosity is controlled by the `RUST_LOG` environment variable
/// (defaults to `info` when unset).  All existing `log::` call sites are
/// automatically forwarded into the tracing pipeline via `LogTracer`.
pub fn init() {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    let log_dir = resolve_log_dir();
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "mesoclaw.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = GUARD.set(guard);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false), // no colour escape codes in log files
        )
        .try_init()
        .ok();

    // Forward all log:: macro call sites into the tracing pipeline.
    tracing_log::LogTracer::init().ok();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "MesoClaw started");
}

fn resolve_log_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    let base = dirs::home_dir().map(|h| h.join("Library").join("Logs"));

    #[cfg(not(target_os = "macos"))]
    let base = dirs::data_local_dir();

    base.map(|d| d.join("com.sprklai.mesoclaw"))
        .unwrap_or_else(std::env::temp_dir)
}
