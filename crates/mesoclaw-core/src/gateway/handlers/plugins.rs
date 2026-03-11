use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::MesoError;
use crate::gateway::state::AppState;
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
) -> Result<Json<InstalledPlugin>, MesoError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| MesoError::PluginNotFound(format!("plugin '{name}' not found")))?;
    Ok(Json(plugin))
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct InstallRequest {
    pub source: String,
    #[serde(default)]
    pub local: bool,
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
) -> Result<(StatusCode, Json<InstalledPlugin>), MesoError> {
    let installed = if req.local {
        state
            .plugin_installer
            .install_from_local(std::path::Path::new(&req.source))
            .await?
    } else {
        state.plugin_installer.install_from_git(&req.source).await?
    };
    Ok((StatusCode::CREATED, Json(installed)))
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
) -> Result<StatusCode, MesoError> {
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
) -> Result<Json<InstalledPlugin>, MesoError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| MesoError::PluginNotFound(format!("plugin '{name}' not found")))?;

    if plugin.enabled {
        state.plugin_registry.disable(&name)?;
    } else {
        state.plugin_registry.enable(&name)?;
    }

    let updated = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| MesoError::Plugin("plugin disappeared after toggle".into()))?;
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
) -> Result<Json<InstalledPlugin>, MesoError> {
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
) -> Result<Json<serde_json::Value>, MesoError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| MesoError::PluginNotFound(format!("plugin '{name}' not found")))?;

    let config_path = plugin.install_path.join("config.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| MesoError::Plugin(format!("read config failed: {e}")))?;
        let config: toml::Value = toml::from_str(&content)
            .map_err(|e| MesoError::Plugin(format!("parse config failed: {e}")))?;
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
) -> Result<Json<serde_json::Value>, MesoError> {
    let plugin = state
        .plugin_registry
        .get(&name)
        .ok_or_else(|| MesoError::PluginNotFound(format!("plugin '{name}' not found")))?;

    let config_path = plugin.install_path.join("config.toml");
    let toml_str = toml::to_string_pretty(&config)
        .map_err(|e| MesoError::Plugin(format!("serialize config failed: {e}")))?;
    std::fs::write(&config_path, toml_str)
        .map_err(|e| MesoError::Plugin(format!("write config failed: {e}")))?;

    Ok(Json(config))
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
            source.join("mesoclaw-plugin.toml"),
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
