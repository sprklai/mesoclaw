//! Tauri IPC commands for module management.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Manager as _, State};

use super::{
    SidecarModule as _,
    manifest::{ModuleInfo, ModuleManifest, ModuleType, ParametersConfig, RuntimeConfig, RuntimeType, SecurityConfig, ServiceConfig},
    ModuleRegistry,
};

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleEntry {
    pub manifest: ModuleManifest,
    pub status: ModuleStatus,
    pub healthy: Option<bool>,
    pub error_message: Option<String>,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// List all registered modules with their health status.
#[tauri::command]
#[tracing::instrument(name = "command.modules.list", skip(registry))]
pub async fn list_modules_command(
    registry: State<'_, Arc<ModuleRegistry>>,
) -> Result<Vec<ModuleEntry>, String> {
    let modules = registry.list();
    let mut entries = Vec::with_capacity(modules.len());
    for module in &modules {
        let (healthy, error_message) = match module.health_check().await {
            Ok(h) => (Some(h), None),
            Err(e) => (None, Some(e)),
        };
        entries.push(ModuleEntry {
            manifest: module.manifest().clone(),
            status: ModuleStatus::Stopped,
            healthy,
            error_message,
        });
    }
    Ok(entries)
}

/// Start a module by id.
#[tauri::command]
#[tracing::instrument(name = "command.modules.start", skip(registry), fields(module_id = %module_id))]
pub async fn start_module_command(
    module_id: String,
    registry: State<'_, Arc<ModuleRegistry>>,
) -> Result<(), String> {
    let module = registry
        .get(&module_id)
        .ok_or_else(|| format!("Module '{module_id}' not found"))?;
    module.start().await
}

/// Stop a module by id.
#[tauri::command]
#[tracing::instrument(name = "command.modules.stop", skip(registry), fields(module_id = %module_id))]
pub async fn stop_module_command(
    module_id: String,
    registry: State<'_, Arc<ModuleRegistry>>,
) -> Result<(), String> {
    let module = registry
        .get(&module_id)
        .ok_or_else(|| format!("Module '{module_id}' not found"))?;
    module.stop().await
}

/// Scaffold a new module by writing a manifest.toml into the modules directory.
#[tauri::command]
#[tracing::instrument(name = "command.modules.create", skip(registry, app), fields(name = %name))]
pub async fn create_module_command(
    name: String,
    module_type: String,
    runtime_type: String,
    command: String,
    description: String,
    app: tauri::AppHandle,
    registry: State<'_, Arc<ModuleRegistry>>,
) -> Result<(), String> {
    let mt: ModuleType = serde_json::from_value(serde_json::Value::String(module_type.clone()))
        .map_err(|_| format!("invalid module_type: {module_type}"))?;
    let rt: RuntimeType = serde_json::from_value(serde_json::Value::String(runtime_type.clone()))
        .map_err(|_| format!("invalid runtime_type: {runtime_type}"))?;

    let id = name.to_lowercase().replace(' ', "-");

    let manifest = ModuleManifest {
        module: ModuleInfo {
            id: id.clone(),
            name,
            version: "1.0.0".to_owned(),
            description,
            module_type: mt,
        },
        runtime: RuntimeConfig {
            runtime_type: rt,
            command,
            args: vec![],
            env: std::collections::HashMap::new(),
            timeout_secs: None,
        },
        security: SecurityConfig::default(),
        parameters: ParametersConfig::default(),
        service: ServiceConfig::default(),
    };

    let toml_content =
        toml::to_string(&manifest).map_err(|e| format!("failed to serialize manifest: {e}"))?;

    let modules_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("cannot resolve app data dir: {e}"))?
        .join("modules");
    std::fs::create_dir_all(&modules_dir)
        .map_err(|e| format!("cannot create modules dir: {e}"))?;

    let module_dir = modules_dir.join(&id);
    if module_dir.exists() {
        return Err(format!("Module '{id}' already exists"));
    }
    std::fs::create_dir_all(&module_dir)
        .map_err(|e| format!("cannot create module dir: {e}"))?;

    std::fs::write(module_dir.join("manifest.toml"), toml_content)
        .map_err(|e| format!("cannot write manifest.toml: {e}"))?;

    registry.reload();
    Ok(())
}
