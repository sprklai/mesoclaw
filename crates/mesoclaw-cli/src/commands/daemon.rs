use std::process::Command;

use crate::client::MesoClient;

pub async fn start() -> Result<(), String> {
    println!("Starting mesoclaw daemon...");

    let result = Command::new("mesoclaw-daemon")
        .spawn()
        .map_err(|e| format!("failed to start daemon: {e}"))?;

    println!("Daemon started (pid: {})", result.id());
    Ok(())
}

pub async fn stop() -> Result<(), String> {
    println!(
        "To stop the daemon, send SIGTERM to the process or use your system's process manager."
    );
    println!("Example: kill $(pgrep mesoclaw-daemon)");
    Ok(())
}

pub async fn status(client: &MesoClient) -> Result<(), String> {
    match client.get::<serde_json::Value>("/health").await {
        Ok(resp) => {
            let status = resp
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!("Daemon status: {status}");
            Ok(())
        }
        Err(e) => {
            println!("Daemon is not reachable: {e}");
            Ok(())
        }
    }
}
