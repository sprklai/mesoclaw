//! Docker container runtime implementation.
//!
//! Uses the Docker CLI (`docker run`) rather than the Docker daemon socket,
//! which avoids the bollard dependency and works in restricted environments
//! where the socket may not be accessible.

use std::process::Stdio;

use async_trait::async_trait;

use super::{ContainerConfig, ContainerRuntime};

// ─── DockerRuntime ────────────────────────────────────────────────────────────

/// Container runtime backed by the `docker` CLI.
pub struct DockerRuntime {
    binary: String,
}

impl DockerRuntime {
    /// Create a new `DockerRuntime` using the given binary path or name.
    pub fn new(binary: String) -> Self {
        Self { binary }
    }

    /// Build the complete argument list for `docker run` from `config`.
    ///
    /// This is exposed as `pub` so that unit tests can inspect the arguments
    /// without needing a live Docker daemon.
    pub fn build_run_args(&self, config: &ContainerConfig) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            // Remove the container automatically when it exits.
            "--rm".to_string(),
            // Keep stdin open so the caller can write to it.
            "-i".to_string(),
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

        // Image name.
        args.push(config.image.clone());

        // Optional entrypoint command override.
        if !config.command.is_empty() {
            args.push(config.command.clone());
        }

        // Arguments forwarded to the command inside the container.
        args.extend(config.args.iter().cloned());

        args
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    fn binary_name(&self) -> &str {
        &self.binary
    }

    fn is_available(&self) -> bool {
        // The binary resolves to a real file or is findable via PATH.
        std::path::Path::new(&self.binary).is_file()
            || which::which(&self.binary).is_ok()
    }

    async fn pull_image(&self, image: &str) -> Result<(), String> {
        let status = tokio::process::Command::new(&self.binary)
            .args(["pull", image])
            .status()
            .await
            .map_err(|e| format!("docker pull failed to start: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "docker pull '{}' exited with status {}",
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
            .map_err(|e| format!("failed to spawn docker container: {e}"))
    }

    async fn stop(&self, container_id: &str) -> Result<(), String> {
        let status = tokio::process::Command::new(&self.binary)
            .args(["stop", container_id])
            .status()
            .await
            .map_err(|e| format!("docker stop failed to start: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "docker stop '{}' exited with status {}",
                container_id, status
            ))
        }
    }
}
