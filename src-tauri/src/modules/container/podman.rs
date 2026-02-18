//! Podman container runtime implementation.
//!
//! Podman is API-compatible with Docker but runs rootless by default —
//! no daemon is required, and containers run as the current user.
//!
//! The key difference from Docker is the addition of `--userns=keep-id`
//! which maps the container UID/GID to the host user, preventing permission
//! issues with volume-mounted directories.

use std::process::Stdio;

use async_trait::async_trait;

use super::{ContainerConfig, ContainerRuntime};

// ─── PodmanRuntime ────────────────────────────────────────────────────────────

/// Container runtime backed by the `podman` CLI.
pub struct PodmanRuntime {
    binary: String,
}

impl PodmanRuntime {
    /// Create a new `PodmanRuntime` using the given binary path or name.
    pub fn new(binary: String) -> Self {
        Self { binary }
    }

    /// Build the complete argument list for `podman run` from `config`.
    ///
    /// Exposed as `pub` for unit-test inspection without a running Podman daemon.
    pub fn build_run_args(&self, config: &ContainerConfig) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-i".to_string(),
            // Rootless Podman: preserve the host user's UID/GID inside the container.
            "--userns=keep-id".to_string(),
        ];

        if config.network_disabled {
            args.push("--network=none".to_string());
        }

        if let Some(mem) = config.memory_limit_mb {
            args.push(format!("--memory={}m", mem));
        }

        for vol in &config.volumes {
            args.push("-v".to_string());
            args.push(vol.clone());
        }

        for (k, v) in &config.env {
            args.push("-e".to_string());
            args.push(format!("{}={}", k, v));
        }

        args.push(config.image.clone());

        if !config.command.is_empty() {
            args.push(config.command.clone());
        }

        args.extend(config.args.iter().cloned());

        args
    }
}

#[async_trait]
impl ContainerRuntime for PodmanRuntime {
    fn binary_name(&self) -> &str {
        &self.binary
    }

    fn is_available(&self) -> bool {
        std::path::Path::new(&self.binary).is_file()
            || which::which(&self.binary).is_ok()
    }

    async fn pull_image(&self, image: &str) -> Result<(), String> {
        let status = tokio::process::Command::new(&self.binary)
            .args(["pull", image])
            .status()
            .await
            .map_err(|e| format!("podman pull failed to start: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "podman pull '{}' exited with status {}",
                image, status
            ))
        }
    }

    async fn spawn(&self, config: &ContainerConfig) -> Result<tokio::process::Child, String> {
        let args = self.build_run_args(config);
        tokio::process::Command::new(&self.binary)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn podman container: {e}"))
    }

    async fn stop(&self, container_id: &str) -> Result<(), String> {
        let status = tokio::process::Command::new(&self.binary)
            .args(["stop", container_id])
            .status()
            .await
            .map_err(|e| format!("podman stop failed to start: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "podman stop '{}' exited with status {}",
                container_id, status
            ))
        }
    }
}
