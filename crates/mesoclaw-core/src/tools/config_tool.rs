use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use serde_json::json;

use crate::config::{AppConfig, save_config};
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

const WHITELISTED_KEYS: &[&str] = &[
    "context_injection_enabled",
    "self_evolution_enabled",
    "learning_enabled",
    "agent_system_prompt",
];

/// Agent tool for reading and updating app configuration.
pub struct ConfigTool {
    config: Arc<ArcSwap<AppConfig>>,
    config_path: PathBuf,
    context_injection_enabled: Arc<AtomicBool>,
    self_evolution_enabled: Arc<AtomicBool>,
}

impl ConfigTool {
    pub fn new(
        config: Arc<ArcSwap<AppConfig>>,
        config_path: PathBuf,
        context_injection_enabled: Arc<AtomicBool>,
        self_evolution_enabled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            config,
            config_path,
            context_injection_enabled,
            self_evolution_enabled,
        }
    }
}

#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "config"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Read or update app configuration. Use 'get' to view current settings (optionally filter by key), 'update' to change whitelisted settings."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["get", "update"],
                    "description": "The config operation to perform"
                },
                "key": {
                    "type": "string",
                    "description": "Config key to get or update"
                },
                "value": {
                    "type": "string",
                    "description": "New value for the key (required for update)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'action' field".into()))?;

        match action {
            "get" => {
                let cfg = self.config.load();
                let config_json = serde_json::to_value(cfg.as_ref()).map_err(|e| {
                    MesoError::Validation(format!("Failed to serialize config: {e}"))
                })?;

                if let Some(key) = args["key"].as_str() {
                    if let Some(value) = config_json.get(key) {
                        Ok(ToolResult::ok(format!("{key} = {value}")))
                    } else {
                        Ok(ToolResult::err(format!("Unknown config key '{key}'")))
                    }
                } else {
                    // Return full config but redact sensitive fields
                    let mut obj = config_json;
                    if let Some(map) = obj.as_object_mut() {
                        map.remove("gateway_auth_token");
                        map.remove("provider_api_key_env");
                    }
                    Ok(ToolResult::ok(
                        serde_json::to_string_pretty(&obj).unwrap_or_default(),
                    ))
                }
            }
            "update" => {
                let key = args["key"]
                    .as_str()
                    .ok_or_else(|| MesoError::Validation("missing 'key' for update".into()))?;
                let value = args["value"]
                    .as_str()
                    .ok_or_else(|| MesoError::Validation("missing 'value' for update".into()))?;

                if !WHITELISTED_KEYS.contains(&key) {
                    return Ok(ToolResult::err(format!(
                        "Key '{key}' is not updatable via this tool. Allowed keys: {}",
                        WHITELISTED_KEYS.join(", ")
                    )));
                }

                match key {
                    "context_injection_enabled" => {
                        let val: bool = value.parse().map_err(|_| {
                            MesoError::Validation("value must be 'true' or 'false'".into())
                        })?;
                        self.context_injection_enabled.store(val, Ordering::Relaxed);
                        Ok(ToolResult::ok(format!(
                            "context_injection_enabled set to {val}"
                        )))
                    }
                    "self_evolution_enabled" => {
                        let val: bool = value.parse().map_err(|_| {
                            MesoError::Validation("value must be 'true' or 'false'".into())
                        })?;
                        self.self_evolution_enabled.store(val, Ordering::Relaxed);
                        Ok(ToolResult::ok(format!(
                            "self_evolution_enabled set to {val}"
                        )))
                    }
                    "learning_enabled" | "agent_system_prompt" => {
                        // These require config file update — mutate a clone and save
                        let mut new_config = (**self.config.load()).clone();
                        match key {
                            "learning_enabled" => {
                                let val: bool = value.parse().map_err(|_| {
                                    MesoError::Validation("value must be 'true' or 'false'".into())
                                })?;
                                new_config.learning_enabled = val;
                            }
                            "agent_system_prompt" => {
                                new_config.agent_system_prompt = Some(value.to_string());
                            }
                            _ => unreachable!(),
                        }
                        save_config(&self.config_path, &new_config)?;
                        self.config.store(Arc::new(new_config));
                        Ok(ToolResult::ok(format!(
                            "{key} updated (saved to config file)"
                        )))
                    }
                    _ => unreachable!(),
                }
            }
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: get, update"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, ConfigTool) {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        let config = Arc::new(ArcSwap::from_pointee(AppConfig::default()));
        let tool = ConfigTool::new(
            config,
            config_path,
            Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(true)),
        );
        (dir, tool)
    }

    // 17.20 — Get full config returns JSON
    #[tokio::test]
    async fn config_tool_get_all() {
        let (_dir, tool) = setup();
        let result = tool.execute(json!({ "action": "get" })).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("gateway_host"));
        // Sensitive fields should be redacted
        assert!(!result.output.contains("gateway_auth_token"));
    }

    // 17.21 — Get specific key returns value
    #[tokio::test]
    async fn config_tool_get_key() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(json!({ "action": "get", "key": "gateway_port" }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("gateway_port"));
        assert!(result.output.contains("18981"));
    }

    // 17.22 — Update whitelisted toggle succeeds
    #[tokio::test]
    async fn config_tool_update_toggle() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(json!({
                "action": "update",
                "key": "context_injection_enabled",
                "value": "false"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("false"));
        assert!(!tool.context_injection_enabled.load(Ordering::Relaxed));
    }

    // 17.23 — Update non-whitelisted key returns error
    #[tokio::test]
    async fn config_tool_update_rejected() {
        let (_dir, tool) = setup();
        let result = tool
            .execute(json!({
                "action": "update",
                "key": "gateway_port",
                "value": "9999"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("not updatable"));
    }

    // 17.24 — Tool name/description/schema validation
    #[test]
    fn config_tool_schema() {
        let (_dir, tool) = setup();
        assert_eq!(tool.name(), "config");
        assert!(tool.description().contains("config"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
    }
}
