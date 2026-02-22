//! Tauri IPC commands for module management.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Manager as _, State};

use super::{
    ModuleRegistry, SidecarModule as _,
    manifest::{
        ModuleInfo, ModuleManifest, ModuleType, ParametersConfig, RuntimeConfig, RuntimeType,
        SecurityConfig, ServiceConfig,
    },
    templates::{self, ModuleTemplate},
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
///
/// # Arguments
/// * `template` - Template type: empty, python_tool, python_ml, python_service, node_tool
/// * `image` - Container image name (required for docker/podman runtime)
/// * `env` - Environment variables as key-value pairs
/// * `volumes` - Volume mounts in "host_path:container_path[:ro]" format
#[tauri::command]
#[tracing::instrument(name = "command.modules.create", skip(registry, app), fields(name = %name))]
pub async fn create_module_command(
    name: String,
    module_type: String,
    runtime_type: String,
    command: String,
    description: String,
    template: Option<String>,
    image: Option<String>,
    env: Option<std::collections::HashMap<String, String>>,
    volumes: Option<Vec<String>>,
    app: tauri::AppHandle,
    registry: State<'_, Arc<ModuleRegistry>>,
) -> Result<(), String> {
    let mt: ModuleType = serde_json::from_value(serde_json::Value::String(module_type.clone()))
        .map_err(|_| format!("invalid module_type: {module_type}"))?;
    let rt: RuntimeType = serde_json::from_value(serde_json::Value::String(runtime_type.clone()))
        .map_err(|_| format!("invalid runtime_type: {runtime_type}"))?;

    // Parse template
    let module_template = match template.as_deref() {
        Some("python_tool") => ModuleTemplate::PythonTool,
        Some("python_ml") => ModuleTemplate::PythonMl,
        Some("python_service") => ModuleTemplate::PythonService,
        Some("node_tool") => ModuleTemplate::NodeTool,
        Some("empty") | None => ModuleTemplate::Empty,
        Some(other) => return Err(format!("invalid template: {other}")),
    };

    let id = name.to_lowercase().replace(' ', "-");
    let is_container = rt != RuntimeType::Native;

    // Determine image
    let final_image = image.or_else(|| {
        if is_container {
            templates::default_image_for_template(&module_template).map(|s| s.to_string())
        } else {
            None
        }
    });

    // Determine command and args based on template
    let (final_command, final_args) = if command.is_empty() {
        let cmd = templates::default_command_for_template(&module_template, is_container);
        let args = templates::default_args_for_template(&module_template);
        (cmd, args)
    } else {
        (command, vec![])
    };

    let manifest = ModuleManifest {
        module: ModuleInfo {
            id: id.clone(),
            name: name.clone(),
            version: "1.0.0".to_owned(),
            description,
            module_type: mt,
        },
        runtime: RuntimeConfig {
            runtime_type: rt,
            command: final_command,
            args: final_args,
            env: env.unwrap_or_default(),
            volumes: volumes.unwrap_or_default(),
            image: final_image,
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
    std::fs::create_dir_all(&modules_dir).map_err(|e| format!("cannot create modules dir: {e}"))?;

    let module_dir = modules_dir.join(&id);
    if module_dir.exists() {
        return Err(format!("Module '{id}' already exists"));
    }
    std::fs::create_dir_all(&module_dir).map_err(|e| format!("cannot create module dir: {e}"))?;

    // Write manifest
    std::fs::write(module_dir.join("manifest.toml"), toml_content)
        .map_err(|e| format!("cannot write manifest.toml: {e}"))?;

    // Generate template files
    let template_files = templates::generate_template_files(
        &module_template,
        &id,
        &name,
        manifest.module.description.as_str(),
        manifest.runtime.image.as_deref(),
    );

    for file in template_files {
        std::fs::write(module_dir.join(&file.filename), file.content)
            .map_err(|e| format!("cannot write {}: {e}", file.filename))?;
    }

    registry.reload();
    Ok(())
}

/// List available module templates.
#[tauri::command]
pub async fn list_module_templates_command() -> Result<Vec<TemplateInfo>, String> {
    Ok(vec![
        TemplateInfo {
            id: "empty".to_string(),
            name: "Empty Module".to_string(),
            description: "Minimal module with just a manifest file".to_string(),
            runtime_types: vec![
                "native".to_string(),
                "docker".to_string(),
                "podman".to_string(),
            ],
            default_image: None,
        },
        TemplateInfo {
            id: "python_tool".to_string(),
            name: "Python Tool".to_string(),
            description: "Python sidecar tool with stdin/stdout JSON-RPC protocol".to_string(),
            runtime_types: vec![
                "native".to_string(),
                "docker".to_string(),
                "podman".to_string(),
            ],
            default_image: Some("python:3.12-slim".to_string()),
        },
        TemplateInfo {
            id: "python_ml".to_string(),
            name: "Python ML".to_string(),
            description: "Python tool with pandas/numpy for data analysis".to_string(),
            runtime_types: vec![
                "native".to_string(),
                "docker".to_string(),
                "podman".to_string(),
            ],
            default_image: Some("python:3.12-slim".to_string()),
        },
        TemplateInfo {
            id: "python_service".to_string(),
            name: "Python HTTP Service".to_string(),
            description: "Long-running Python HTTP service with /health and /execute endpoints"
                .to_string(),
            runtime_types: vec![
                "native".to_string(),
                "docker".to_string(),
                "podman".to_string(),
            ],
            default_image: Some("python:3.12-slim".to_string()),
        },
        TemplateInfo {
            id: "node_tool".to_string(),
            name: "Node.js Tool".to_string(),
            description: "Node.js sidecar tool with stdin/stdout JSON-RPC protocol".to_string(),
            runtime_types: vec![
                "native".to_string(),
                "docker".to_string(),
                "podman".to_string(),
            ],
            default_image: Some("node:20-slim".to_string()),
        },
    ])
}

/// Template information for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub runtime_types: Vec<String>,
    pub default_image: Option<String>,
}
