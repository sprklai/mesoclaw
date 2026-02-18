//! Container runtime abstraction for sidecar modules.
//!
//! When a module manifest specifies `runtime.type = "docker"` or `"podman"`,
//! the module is executed inside a container instead of as a native process.
//! The container shares the same newline-delimited JSON stdio protocol as
//! native sidecars.
//!
//! # Auto-detection order
//! 1. Podman — preferred (rootless by default, no daemon required)
//! 2. Docker — fallback
//! 3. `None` if neither is found in `$PATH`
//!
//! Feature-gated: only compiled with `--features containers`.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

pub mod docker;
pub mod podman;

pub use docker::DockerRuntime;
pub use podman::PodmanRuntime;

// ─── ContainerConfig ──────────────────────────────────────────────────────────

/// Configuration for running a sidecar module inside a container.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Container image name (e.g. `"ghcr.io/my-org/my-tool:1.0"`).
    pub image: String,
    /// Entrypoint command inside the container (may be empty to use image default).
    pub command: String,
    /// Arguments passed to the command inside the container.
    pub args: Vec<String>,
    /// Extra environment variables injected into the container.
    pub env: HashMap<String, String>,
    /// Volume mounts in `"host_path:container_path"` format.
    pub volumes: Vec<String>,
    /// Memory limit in megabytes.  `None` means no limit imposed by us.
    pub memory_limit_mb: Option<u64>,
    /// When `true` the container starts with `--network=none`.
    pub network_disabled: bool,
    /// Wall-clock timeout for the container run (seconds).
    pub timeout_secs: Option<u64>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            image: String::new(),
            command: String::new(),
            args: vec![],
            env: HashMap::new(),
            volumes: vec![],
            memory_limit_mb: None,
            network_disabled: false,
            timeout_secs: Some(30),
        }
    }
}

// ─── ContainerRuntime trait ───────────────────────────────────────────────────

/// Abstraction over container runtimes (Docker, Podman, …).
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Return the CLI binary name used by this runtime (e.g. `"docker"`).
    fn binary_name(&self) -> &str;

    /// Return `true` if the runtime binary is reachable on the current system.
    fn is_available(&self) -> bool;

    /// Pull `image` if it is not already present locally.
    async fn pull_image(&self, image: &str) -> Result<(), String>;

    /// Spawn the container as an interactive child process with stdio piped.
    ///
    /// The returned `Child` behaves identically to a native sidecar process —
    /// stdin/stdout are piped and the caller drives the stdio JSON protocol.
    async fn spawn(&self, config: &ContainerConfig) -> Result<tokio::process::Child, String>;

    /// Stop a running container by its ID.
    async fn stop(&self, container_id: &str) -> Result<(), String>;
}

// ─── Auto-detection ───────────────────────────────────────────────────────────

/// Detect the best available container runtime.
///
/// Checks Podman first (rootless = smaller attack surface), then Docker.
/// Returns `None` if neither binary is found in `$PATH`.
pub fn detect_runtime() -> Option<Arc<dyn ContainerRuntime>> {
    if let Ok(path) = which::which("podman") {
        let binary = path.to_string_lossy().into_owned();
        return Some(Arc::new(PodmanRuntime::new(binary)));
    }
    if let Ok(path) = which::which("docker") {
        let binary = path.to_string_lossy().into_owned();
        return Some(Arc::new(DockerRuntime::new(binary)));
    }
    None
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_config_defaults() {
        let cfg = ContainerConfig::default();
        assert!(cfg.image.is_empty());
        assert!(cfg.command.is_empty());
        assert!(cfg.args.is_empty());
        assert!(cfg.env.is_empty());
        assert!(cfg.volumes.is_empty());
        assert_eq!(cfg.memory_limit_mb, None);
        assert!(!cfg.network_disabled);
        assert_eq!(cfg.timeout_secs, Some(30));
    }

    #[test]
    fn container_config_all_fields() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "val".to_string());

        let cfg = ContainerConfig {
            image: "ubuntu:22.04".to_string(),
            command: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), "echo hello".to_string()],
            env,
            volumes: vec!["/host:/container:ro".to_string()],
            memory_limit_mb: Some(256),
            network_disabled: true,
            timeout_secs: Some(60),
        };

        assert_eq!(cfg.image, "ubuntu:22.04");
        assert_eq!(cfg.args.len(), 2);
        assert!(cfg.network_disabled);
        assert_eq!(cfg.memory_limit_mb, Some(256));
        assert_eq!(cfg.volumes.len(), 1);
        assert_eq!(cfg.timeout_secs, Some(60));
    }

    #[test]
    fn detect_runtime_does_not_panic() {
        // The function should run without panicking in any environment.
        // Whether it returns Some or None depends on installed binaries.
        let _result = detect_runtime();
    }

    #[test]
    fn docker_runtime_binary_name() {
        let rt = DockerRuntime::new("docker".to_string());
        assert_eq!(rt.binary_name(), "docker");
    }

    #[test]
    fn podman_runtime_binary_name() {
        let rt = PodmanRuntime::new("podman".to_string());
        assert_eq!(rt.binary_name(), "podman");
    }

    #[test]
    fn docker_runtime_unavailable_for_fake_binary() {
        let rt = DockerRuntime::new("/nonexistent/docker-xyzzy-fake".to_string());
        assert!(!rt.is_available());
    }

    #[test]
    fn podman_runtime_unavailable_for_fake_binary() {
        let rt = PodmanRuntime::new("/nonexistent/podman-xyzzy-fake".to_string());
        assert!(!rt.is_available());
    }

    #[test]
    fn docker_builds_minimal_run_args() {
        let rt = DockerRuntime::new("docker".to_string());
        let cfg = ContainerConfig {
            image: "alpine:3".to_string(),
            command: "sh".to_string(),
            ..ContainerConfig::default()
        };
        let args = rt.build_run_args(&cfg);

        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--rm".to_string()));
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"alpine:3".to_string()));
        assert!(args.contains(&"sh".to_string()));
        // No network/memory flags for default config.
        assert!(!args.iter().any(|a| a.starts_with("--memory=")));
        assert!(!args.iter().any(|a| a.starts_with("--network=")));
    }

    #[test]
    fn docker_builds_memory_and_network_flags() {
        let rt = DockerRuntime::new("docker".to_string());
        let cfg = ContainerConfig {
            image: "alpine:3".to_string(),
            command: "sh".to_string(),
            memory_limit_mb: Some(128),
            network_disabled: true,
            ..ContainerConfig::default()
        };
        let args = rt.build_run_args(&cfg);

        assert!(args.iter().any(|a| a == "--memory=128m"));
        assert!(args.iter().any(|a| a == "--network=none"));
    }

    #[test]
    fn docker_builds_volume_flags() {
        let rt = DockerRuntime::new("docker".to_string());
        let cfg = ContainerConfig {
            image: "alpine:3".to_string(),
            command: "sh".to_string(),
            volumes: vec!["/tmp/host:/tmp/container:ro".to_string()],
            ..ContainerConfig::default()
        };
        let args = rt.build_run_args(&cfg);

        let v_pos = args
            .iter()
            .position(|a| a == "-v")
            .expect("-v flag should be present");
        assert_eq!(args[v_pos + 1], "/tmp/host:/tmp/container:ro");
    }

    #[test]
    fn podman_includes_userns_flag() {
        let rt = PodmanRuntime::new("podman".to_string());
        let cfg = ContainerConfig {
            image: "alpine:3".to_string(),
            command: "sh".to_string(),
            ..ContainerConfig::default()
        };
        let args = rt.build_run_args(&cfg);

        // Podman adds --userns=keep-id for rootless operation.
        assert!(args.iter().any(|a| a.starts_with("--userns=")));
    }

    #[test]
    fn docker_and_podman_use_different_binary_names() {
        let docker = DockerRuntime::new("docker".to_string());
        let podman = PodmanRuntime::new("podman".to_string());
        assert_ne!(docker.binary_name(), podman.binary_name());
    }
}
