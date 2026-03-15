use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::ZeniiError;
use crate::gateway::state::AppState;
use crate::plugins::PluginManifest;
use crate::plugins::registry::InstalledPlugin;

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct PluginListItem {
    name: String,
    version: String,
    description: String,
    enabled: bool,
    tools_count: usize,
    skills_count: usize,
}

impl From<&InstalledPlugin> for PluginListItem {
    fn from(p: &InstalledPlugin) -> Self {
        Self {
            name: p.manifest.plugin.name.clone(),
            version: p.manifest.plugin.version.clone(),
            description: p.manifest.plugin.description.clone(),
            enabled: p.enabled,
            tools_count: p.manifest.tools.len(),
            skills_count: p.manifest.skills.len(),
        }
    }
}

/// GET /plugins — List all installed plugins.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/plugins", tag = "Plugins",
    responses((status = 200, description = "List of installed plugins", body = Vec<PluginListItem>))
))]
pub async fn list_plugins(State(state): State<Arc<AppState>>) -> Json<Vec<PluginListItem>> {
    let plugins = state.plugin_registry.list();
    let items: Vec<PluginListItem> = plugins.iter().map(PluginListItem::from).collect();
    Json(items)
}

/// GET /plugins/{name} — Get plugin details.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/plugins/{name}", tag = "Plugins",
    params(("name" = String, Path, description = "Plugin name")),
    responses(
        (status = 200, description = "Plugin details"),
        (status = 404, description = "Plugin not found")
    )
))]
pub async fn get_plugin(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<InstalledPlugin>, ZeniiError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;
    Ok(Json(plugin))
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct InstallRequest {
    pub source: String,
    #[serde(default)]
    pub local: bool,
    /// Install all plugins found in a local directory
    #[serde(default)]
    pub all: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InstallResponse {
    Single(Box<InstalledPlugin>),
    Batch(Vec<InstalledPlugin>),
}

/// POST /plugins/install — Install a plugin from source.
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/plugins/install", tag = "Plugins",
    request_body = InstallRequest,
    responses(
        (status = 201, description = "Plugin installed"),
        (status = 400, description = "Invalid plugin source")
    )
))]
pub async fn install_plugin(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InstallRequest>,
) -> Result<(StatusCode, Json<InstallResponse>), ZeniiError> {
    if req.local && req.all {
        let installed = state
            .plugin_installer
            .install_all_from_local(std::path::Path::new(&req.source))
            .await?;
        Ok((StatusCode::CREATED, Json(InstallResponse::Batch(installed))))
    } else if req.local {
        let installed = state
            .plugin_installer
            .install_from_local(std::path::Path::new(&req.source))
            .await?;
        Ok((
            StatusCode::CREATED,
            Json(InstallResponse::Single(Box::new(installed))),
        ))
    } else {
        let installed = state.plugin_installer.install_from_git(&req.source).await?;
        Ok((
            StatusCode::CREATED,
            Json(InstallResponse::Single(Box::new(installed))),
        ))
    }
}

/// DELETE /plugins/{name} — Uninstall a plugin.
#[cfg_attr(feature = "api-docs", utoipa::path(
    delete, path = "/plugins/{name}", tag = "Plugins",
    params(("name" = String, Path, description = "Plugin name")),
    responses(
        (status = 204, description = "Plugin removed"),
        (status = 404, description = "Plugin not found")
    )
))]
pub async fn remove_plugin(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<StatusCode, ZeniiError> {
    state.plugin_installer.remove(&name).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /plugins/{name}/toggle — Enable or disable a plugin.
#[cfg_attr(feature = "api-docs", utoipa::path(
    put, path = "/plugins/{name}/toggle", tag = "Plugins",
    params(("name" = String, Path, description = "Plugin name")),
    responses(
        (status = 200, description = "Plugin toggled"),
        (status = 404, description = "Plugin not found")
    )
))]
pub async fn toggle_plugin(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<InstalledPlugin>, ZeniiError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;

    if plugin.enabled {
        state.plugin_registry.disable(&name)?;
    } else {
        state.plugin_registry.enable(&name)?;
    }

    let updated = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| ZeniiError::Plugin("plugin disappeared after toggle".into()))?;
    Ok(Json(updated))
}

/// POST /plugins/{name}/update — Update plugin to latest.
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/plugins/{name}/update", tag = "Plugins",
    params(("name" = String, Path, description = "Plugin name")),
    responses(
        (status = 200, description = "Plugin updated"),
        (status = 404, description = "Plugin not found")
    )
))]
pub async fn update_plugin(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<InstalledPlugin>, ZeniiError> {
    let installed = state.plugin_installer.update(&name).await?;
    Ok(Json(installed))
}

/// GET /plugins/{name}/config — Get plugin config.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/plugins/{name}/config", tag = "Plugins",
    params(("name" = String, Path, description = "Plugin name")),
    responses(
        (status = 200, description = "Plugin configuration"),
        (status = 404, description = "Plugin not found")
    )
))]
pub async fn get_plugin_config(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ZeniiError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;

    let config_path = plugin.install_path.join("config.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| ZeniiError::Plugin(format!("read config failed: {e}")))?;
        let config: toml::Value = toml::from_str(&content)
            .map_err(|e| ZeniiError::Plugin(format!("parse config failed: {e}")))?;
        Ok(Json(serde_json::to_value(config).unwrap_or_default()))
    } else {
        Ok(Json(serde_json::json!({})))
    }
}

/// PUT /plugins/{name}/config — Update plugin config.
#[cfg_attr(feature = "api-docs", utoipa::path(
    put, path = "/plugins/{name}/config", tag = "Plugins",
    params(("name" = String, Path, description = "Plugin name")),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Plugin configuration updated"),
        (status = 404, description = "Plugin not found")
    )
))]
pub async fn update_plugin_config(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ZeniiError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;

    let config_path = plugin.install_path.join("config.toml");
    let toml_str = toml::to_string_pretty(&config)
        .map_err(|e| ZeniiError::Plugin(format!("serialize config failed: {e}")))?;
    std::fs::write(&config_path, toml_str)
        .map_err(|e| ZeniiError::Plugin(format!("write config failed: {e}")))?;

    Ok(Json(config))
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct AvailablePlugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub tools_count: usize,
    pub skills_count: usize,
    pub installed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct AvailablePluginsResponse {
    pub repo_url: String,
    pub plugins: Vec<AvailablePlugin>,
}

/// GET /plugins/available — List plugins from official repo.
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/plugins/available", tag = "Plugins",
    responses(
        (status = 200, description = "List of available plugins from official repo", body = AvailablePluginsResponse),
        (status = 500, description = "Failed to fetch plugin catalog")
    )
))]
pub async fn list_available_plugins(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AvailablePluginsResponse>, ZeniiError> {
    let config = state.config.load();
    let repo_url = &config.official_plugins_repo;

    // Clone to temp dir (shallow)
    let temp_path = std::env::temp_dir().join(format!(
        "zenii-browse-plugins-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));

    let output = tokio::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            repo_url,
            temp_path.to_str().unwrap_or("."),
        ])
        .output()
        .await
        .map_err(|e| ZeniiError::Plugin(format!("git clone failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_dir_all(&temp_path);
        return Err(ZeniiError::Plugin(format!(
            "failed to fetch plugin catalog: {stderr}"
        )));
    }

    // Scan plugins/ subdirectory for manifests
    let plugins_root = temp_path.join("plugins");
    let scan_dir = if plugins_root.is_dir() {
        &plugins_root
    } else {
        &temp_path
    };

    let mut available = Vec::new();
    if let Ok(entries) = std::fs::read_dir(scan_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("zenii-plugin.toml");
            if !manifest_path.exists() {
                continue;
            }
            if let Ok(manifest) = PluginManifest::from_file(&manifest_path) {
                let name = manifest.plugin.name.clone();
                let installed = state.plugin_registry.get(&name).is_some();
                available.push(AvailablePlugin {
                    name,
                    version: manifest.plugin.version,
                    description: manifest.plugin.description,
                    author: manifest.plugin.author,
                    tools_count: manifest.tools.len(),
                    skills_count: manifest.skills.len(),
                    installed,
                });
            }
        }
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_path);

    available.sort_by(|a, b| a.name.cmp(&b.name));
    info!(
        "Fetched {} available plugins from official repo",
        available.len()
    );

    Ok(Json(AvailablePluginsResponse {
        repo_url: repo_url.clone(),
        plugins: available,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn test_app() -> (tempfile::TempDir, axum::Router) {
        let (dir, state) = crate::gateway::handlers::tests::test_state().await;
        let app = crate::gateway::routes::build_router(state);
        (dir, app)
    }

    // 9.0.20 — Gateway GET /plugins
    #[tokio::test]
    async fn get_plugins_empty() {
        let (_dir, app) = test_app().await;
        let req = Request::builder()
            .uri("/plugins")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let plugins: Vec<PluginListItem> = serde_json::from_slice(&body).unwrap();
        assert!(plugins.is_empty());
    }

    // 9.0.21 — Gateway POST /plugins/install (local)
    #[tokio::test]
    async fn install_plugin_local() {
        let (dir, app) = test_app().await;

        // Create a plugin source directory
        let source = dir.path().join("plugin-source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("zenii-plugin.toml"),
            r#"[plugin]
name = "test-plugin"
version = "1.0.0"
description = "Test plugin"
"#,
        )
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/plugins/install")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&InstallRequest {
                    source: source.to_string_lossy().to_string(),
                    local: true,
                    all: false,
                })
                .unwrap(),
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // 9.0.22 — Gateway DELETE /plugins/{name}
    #[tokio::test]
    async fn delete_nonexistent_plugin() {
        let (_dir, app) = test_app().await;
        let req = Request::builder()
            .method("DELETE")
            .uri("/plugins/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // 9.0.23 — Gateway PUT /plugins/{name}/toggle
    #[tokio::test]
    async fn toggle_nonexistent_plugin() {
        let (_dir, app) = test_app().await;
        let req = Request::builder()
            .method("PUT")
            .uri("/plugins/nonexistent/toggle")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
