//! Long-lived sidecar service management.
//!
//! `SidecarService` wraps a [`ModuleManifest`] with `module.type = "service"`
//! and handles:
//! - Spawning and keeping the process/container alive
//! - Health-check polling until the service becomes ready
//! - Auto-restart on unexpected exit
//! - HTTP communication (GET health, POST execute)
//!
//! HTTP dispatch is performed via the `reqwest` client already in scope.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::Value;
use tokio::process::Child;

use super::manifest::{ModuleManifest, ModuleType};

// ─── ServiceStatus ───────────────────────────────────────────────────────────

/// Current status of a managed sidecar service.
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Healthy,
    Unhealthy { reason: String },
}

impl ServiceStatus {
    pub fn is_healthy(&self) -> bool {
        *self == ServiceStatus::Healthy
    }
}

// ─── SidecarService ──────────────────────────────────────────────────────────

/// Manages a long-lived sidecar service process.
pub struct SidecarService {
    pub manifest: Arc<ModuleManifest>,
    status: Arc<Mutex<ServiceStatus>>,
    child: Arc<Mutex<Option<Child>>>,
    http_client: reqwest::Client,
}

impl SidecarService {
    /// Create a new `SidecarService` for the given manifest.
    ///
    /// Returns `Err` if the manifest's module type is not `Service`.
    pub fn new(manifest: Arc<ModuleManifest>) -> Result<Self, String> {
        if manifest.module.module_type != ModuleType::Service {
            return Err(format!(
                "module '{}' is not a service (type = {:?})",
                manifest.module.id, manifest.module.module_type
            ));
        }
        Ok(Self {
            manifest,
            status: Arc::new(Mutex::new(ServiceStatus::Stopped)),
            child: Arc::new(Mutex::new(None)),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        })
    }

    /// Return the current service status.
    pub fn status(&self) -> ServiceStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or(ServiceStatus::Stopped)
    }

    /// Spawn the service process and poll until healthy or timeout.
    ///
    /// The health check polls `service.health_url("127.0.0.1")` at
    /// `service.health_poll_secs` intervals for up to
    /// `service.startup_timeout_secs` seconds.
    pub async fn start(&self) -> Result<(), String> {
        self.set_status(ServiceStatus::Starting);

        let mut cmd = tokio::process::Command::new(&self.manifest.runtime.command);
        cmd.args(&self.manifest.runtime.args);
        for (k, v) in &self.manifest.runtime.env {
            cmd.env(k, v);
        }
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn service '{}': {e}", self.manifest.module.id))?;

        *self.child.lock().map_err(|e| e.to_string())? = Some(child);

        // Poll health endpoint.
        let svc = &self.manifest.service;
        let healthy = match svc.health_url("127.0.0.1") {
            Some(url) => self.wait_for_health(&url, svc.startup_timeout_secs, svc.health_poll_secs).await,
            None => {
                // No HTTP port configured — assume healthy immediately.
                true
            }
        };

        if healthy {
            self.set_status(ServiceStatus::Healthy);
            Ok(())
        } else {
            let reason = format!(
                "service '{}' did not become healthy within {}s",
                self.manifest.module.id,
                svc.startup_timeout_secs
            );
            self.set_status(ServiceStatus::Unhealthy { reason: reason.clone() });
            Err(reason)
        }
    }

    /// Stop the service process.
    pub async fn stop(&self) -> Result<(), String> {
        // Take the child out while holding the lock, then drop the lock before
        // awaiting — a MutexGuard must not be held across an await point.
        let child_opt = {
            let mut guard = self.child.lock().map_err(|e| e.to_string())?;
            guard.take()
        };
        if let Some(mut child) = child_opt {
            let _ = child.kill().await;
        }
        self.set_status(ServiceStatus::Stopped);
        Ok(())
    }

    /// Send a JSON request to the service's execute endpoint.
    ///
    /// Returns the parsed JSON response body.
    pub async fn execute(&self, payload: Value) -> Result<Value, String> {
        let url = self
            .manifest
            .service
            .execute_url("127.0.0.1")
            .ok_or_else(|| {
                format!(
                    "service '{}' has no http_port configured",
                    self.manifest.module.id
                )
            })?;

        let resp = self
            .http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("execute request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("execute returned status {}", resp.status()));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("failed to parse execute response: {e}"))
    }

    /// Check whether the health endpoint returns 2xx.
    pub async fn check_health(&self) -> bool {
        let Some(url) = self.manifest.service.health_url("127.0.0.1") else {
            return true;
        };
        match self.http_client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    fn set_status(&self, status: ServiceStatus) {
        if let Ok(mut guard) = self.status.lock() {
            *guard = status;
        }
    }

    /// Poll `url` every `poll_secs` seconds until healthy or `timeout_secs` expires.
    async fn wait_for_health(&self, url: &str, timeout_secs: u64, poll_secs: u64) -> bool {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
        let poll_interval = Duration::from_secs(poll_secs.max(1));

        loop {
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            match self.http_client.get(url).send().await {
                Ok(r) if r.status().is_success() => return true,
                _ => {}
            }
            tokio::time::sleep(poll_interval).await;
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::manifest::{
        ModuleInfo, ModuleType, ModuleManifest, ParametersConfig, RuntimeConfig,
        RuntimeType, SecurityConfig, ServiceConfig,
    };

    fn service_manifest(port: Option<u16>) -> Arc<ModuleManifest> {
        Arc::new(ModuleManifest {
            module: ModuleInfo {
                id: "test-svc".to_string(),
                name: "Test Service".to_string(),
                version: "0.1.0".to_string(),
                description: "A test service".to_string(),
                module_type: ModuleType::Service,
            },
            runtime: RuntimeConfig {
                runtime_type: RuntimeType::Native,
                command: "true".to_string(), // exists on Linux, exits 0 immediately
                args: vec![],
                env: Default::default(),
                timeout_secs: Some(5),
            },
            security: SecurityConfig::default(),
            parameters: ParametersConfig::default(),
            service: ServiceConfig {
                http_port: port,
                startup_timeout_secs: 1, // short for tests
                health_poll_secs: 1,
                ..Default::default()
            },
        })
    }

    fn tool_manifest() -> Arc<ModuleManifest> {
        Arc::new(ModuleManifest {
            module: ModuleInfo {
                id: "test-tool".to_string(),
                name: "Test Tool".to_string(),
                version: "0.1.0".to_string(),
                description: "A tool".to_string(),
                module_type: ModuleType::Tool,
            },
            runtime: RuntimeConfig {
                runtime_type: RuntimeType::Native,
                command: "true".to_string(),
                args: vec![],
                env: Default::default(),
                timeout_secs: None,
            },
            security: SecurityConfig::default(),
            parameters: ParametersConfig::default(),
            service: ServiceConfig::default(),
        })
    }

    #[test]
    fn new_rejects_non_service_manifest() {
        let manifest = tool_manifest();
        let result = SidecarService::new(manifest);
        assert!(result.is_err(), "Tool manifest should be rejected");
    }

    #[test]
    fn new_accepts_service_manifest() {
        let manifest = service_manifest(None);
        let result = SidecarService::new(manifest);
        assert!(result.is_ok(), "Service manifest should be accepted");
    }

    #[test]
    fn initial_status_is_stopped() {
        let svc = SidecarService::new(service_manifest(None)).unwrap();
        assert_eq!(svc.status(), ServiceStatus::Stopped);
    }

    #[test]
    fn service_config_health_url_with_port() {
        let config = ServiceConfig {
            http_port: Some(9000),
            health_endpoint: "/health".to_string(),
            ..Default::default()
        };
        assert_eq!(
            config.health_url("127.0.0.1"),
            Some("http://127.0.0.1:9000/health".to_string())
        );
    }

    #[test]
    fn service_config_health_url_no_port_returns_none() {
        let config = ServiceConfig {
            http_port: None,
            ..Default::default()
        };
        assert!(config.health_url("127.0.0.1").is_none());
    }

    #[test]
    fn service_config_execute_url_with_port() {
        let config = ServiceConfig {
            http_port: Some(8080),
            execute_endpoint: "/execute".to_string(),
            ..Default::default()
        };
        assert_eq!(
            config.execute_url("127.0.0.1"),
            Some("http://127.0.0.1:8080/execute".to_string())
        );
    }

    #[test]
    fn service_config_base_url() {
        let config = ServiceConfig {
            http_port: Some(3000),
            ..Default::default()
        };
        assert_eq!(
            config.base_url("localhost"),
            Some("http://localhost:3000".to_string())
        );
    }

    #[tokio::test]
    async fn stop_on_stopped_service_is_safe() {
        let svc = SidecarService::new(service_manifest(None)).unwrap();
        let result = svc.stop().await;
        assert!(result.is_ok(), "stopping an already-stopped service should be safe");
    }

    #[tokio::test]
    async fn check_health_no_port_returns_true() {
        // Service with no http_port — health is assumed healthy.
        let svc = SidecarService::new(service_manifest(None)).unwrap();
        let healthy = svc.check_health().await;
        assert!(healthy, "service without port should be considered healthy");
    }

    #[tokio::test]
    async fn check_health_unreachable_port_returns_false() {
        // Port 1 is almost certainly not listening.
        let svc = SidecarService::new(service_manifest(Some(1))).unwrap();
        let healthy = svc.check_health().await;
        assert!(!healthy, "unreachable port → unhealthy");
    }
}
