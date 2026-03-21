use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::security::RiskLevel;
use crate::tools::ToolRegistry;
use crate::tools::traits::Tool;

/// Permission state for a tool on a surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PermissionState {
    Allowed,
    Denied,
    /// Phase 2: ask user once, remember the answer.
    AskOnce,
    /// Phase 2: ask user every time.
    AskAlways,
}

/// Risk-level defaults + per-surface per-tool overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ToolPermissions {
    pub low_risk_default: PermissionState,
    pub medium_risk_default: PermissionState,
    pub high_risk_default: PermissionState,
    /// Per-surface, per-tool overrides: surface_name -> { tool_name -> state }
    pub overrides: HashMap<String, HashMap<String, PermissionState>>,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        let high_risk_tools: Vec<(&str, PermissionState)> = vec![
            ("shell", PermissionState::Allowed),
            ("file_read", PermissionState::Allowed),
            ("file_write", PermissionState::Allowed),
            ("file_list", PermissionState::Allowed),
            ("file_search", PermissionState::Allowed),
            ("patch", PermissionState::Allowed),
            ("process", PermissionState::Allowed),
        ];

        let local_overrides: HashMap<String, PermissionState> = high_risk_tools
            .iter()
            .map(|(name, state)| ((*name).to_string(), state.clone()))
            .collect();

        Self {
            low_risk_default: PermissionState::Allowed,
            medium_risk_default: PermissionState::Allowed,
            high_risk_default: PermissionState::Denied,
            overrides: HashMap::from([
                ("desktop".into(), local_overrides.clone()),
                ("cli".into(), local_overrides.clone()),
                ("tui".into(), local_overrides),
            ]),
        }
    }
}

/// Info about a single tool's permission on a surface (for API/UI).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolPermissionInfo {
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub state: PermissionState,
    pub is_override: bool,
}

/// Resolves tool permissions against config.
pub struct PermissionResolver;

impl PermissionResolver {
    /// Resolve permission for a tool on a surface.
    pub fn resolve(
        config: &ToolPermissions,
        tool_name: &str,
        risk_level: RiskLevel,
        surface: &str,
    ) -> PermissionState {
        // 1. Check per-surface, per-tool override
        if let Some(surface_overrides) = config.overrides.get(surface)
            && let Some(state) = surface_overrides.get(tool_name)
        {
            return state.clone();
        }

        // 2. Fall back to risk-level default
        match risk_level {
            RiskLevel::Low => config.low_risk_default.clone(),
            RiskLevel::Medium => config.medium_risk_default.clone(),
            RiskLevel::High => config.high_risk_default.clone(),
        }
    }

    /// Get all tools allowed on a surface from a registry.
    pub fn allowed_tools(
        config: &ToolPermissions,
        surface: &str,
        registry: &ToolRegistry,
    ) -> Vec<Arc<dyn Tool>> {
        registry
            .to_vec()
            .into_iter()
            .filter(|tool| {
                matches!(
                    Self::resolve(config, tool.name(), tool.risk_level(), surface),
                    PermissionState::Allowed
                )
            })
            .collect()
    }

    /// Get all executable tools on a surface (Allowed + AskOnce + AskAlways).
    /// Excludes only Denied tools. AskOnce/AskAlways tools are included so the
    /// agent can attempt to use them and trigger the approval gate.
    pub fn executable_tools(
        config: &ToolPermissions,
        surface: &str,
        registry: &ToolRegistry,
    ) -> Vec<Arc<dyn Tool>> {
        registry
            .to_vec()
            .into_iter()
            .filter(|tool| {
                !matches!(
                    Self::resolve(config, tool.name(), tool.risk_level(), surface),
                    PermissionState::Denied
                )
            })
            .collect()
    }

    /// Get permission summary for all tools on a surface (for UI/API).
    pub fn list_permissions(
        config: &ToolPermissions,
        surface: &str,
        registry: &ToolRegistry,
    ) -> Vec<ToolPermissionInfo> {
        registry
            .to_vec()
            .into_iter()
            .map(|tool| ToolPermissionInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                risk_level: tool.risk_level(),
                state: Self::resolve(config, tool.name(), tool.risk_level(), surface),
                is_override: config
                    .overrides
                    .get(surface)
                    .is_some_and(|o| o.contains_key(tool.name())),
            })
            .collect()
    }

    /// Get the names of all allowed tools on a surface.
    pub fn allowed_tool_names(
        config: &ToolPermissions,
        surface: &str,
        registry: &ToolRegistry,
    ) -> Vec<String> {
        registry
            .to_vec()
            .into_iter()
            .filter(|tool| {
                matches!(
                    Self::resolve(config, tool.name(), tool.risk_level(), surface),
                    PermissionState::Allowed
                )
            })
            .map(|tool| tool.name().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // P19.1 — Low risk tool with no override returns low_risk_default
    #[test]
    fn resolve_no_override_low() {
        let config = ToolPermissions::default();
        let state = PermissionResolver::resolve(&config, "web_search", RiskLevel::Low, "telegram");
        assert_eq!(state, PermissionState::Allowed);
    }

    // P19.2 — High risk tool with no override returns high_risk_default (Denied)
    #[test]
    fn resolve_no_override_high() {
        let config = ToolPermissions::default();
        let state = PermissionResolver::resolve(&config, "shell", RiskLevel::High, "telegram");
        assert_eq!(state, PermissionState::Denied);
    }

    // P19.3 — Surface-specific override takes precedence
    #[test]
    fn resolve_surface_override() {
        let config = ToolPermissions::default();
        // Desktop has shell=Allowed override
        let state = PermissionResolver::resolve(&config, "shell", RiskLevel::High, "desktop");
        assert_eq!(state, PermissionState::Allowed);
    }

    // P19.4 — Medium risk tool returns medium_risk_default
    #[test]
    fn resolve_medium_default() {
        let config = ToolPermissions::default();
        let state = PermissionResolver::resolve(&config, "memory", RiskLevel::Medium, "telegram");
        assert_eq!(state, PermissionState::Allowed);
    }

    // P19.5 — allowed_tools filters correctly
    #[test]
    fn allowed_tools_filters() {
        let config = ToolPermissions::default();
        let registry = ToolRegistry::new();

        // Register a low-risk tool and a high-risk tool
        registry
            .register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))
            .unwrap();
        registry
            .register(Arc::new(crate::tools::shell::ShellTool::new(
                Arc::new(crate::security::SecurityPolicy::default_policy()),
                30,
            )))
            .unwrap();

        // On telegram: system_info (Low=Allowed), shell (High=Denied)
        let allowed = PermissionResolver::allowed_tools(&config, "telegram", &registry);
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].name(), "system_info");

        // On desktop: both allowed (shell has override)
        let allowed = PermissionResolver::allowed_tools(&config, "desktop", &registry);
        assert_eq!(allowed.len(), 2);
    }

    // P19.6 — list_permissions returns info for all tools
    #[test]
    fn list_permissions_all() {
        let config = ToolPermissions::default();
        let registry = ToolRegistry::new();
        registry
            .register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))
            .unwrap();

        let perms = PermissionResolver::list_permissions(&config, "telegram", &registry);
        assert_eq!(perms.len(), 1);
        assert_eq!(perms[0].name, "system_info");
        assert_eq!(perms[0].state, PermissionState::Allowed);
        assert!(!perms[0].is_override);
    }

    // P19.7 — Default ToolPermissions has correct risk defaults
    #[test]
    fn default_permissions() {
        let config = ToolPermissions::default();
        assert_eq!(config.low_risk_default, PermissionState::Allowed);
        assert_eq!(config.medium_risk_default, PermissionState::Allowed);
        assert_eq!(config.high_risk_default, PermissionState::Denied);
    }

    // P19.8 — Desktop surface overrides all high-risk to Allowed
    #[test]
    fn default_desktop_overrides() {
        let config = ToolPermissions::default();
        let desktop = config.overrides.get("desktop").unwrap();
        assert_eq!(desktop.get("shell"), Some(&PermissionState::Allowed));
        assert_eq!(desktop.get("file_read"), Some(&PermissionState::Allowed));
        assert_eq!(desktop.get("file_write"), Some(&PermissionState::Allowed));
        assert_eq!(desktop.get("file_list"), Some(&PermissionState::Allowed));
        assert_eq!(desktop.get("file_search"), Some(&PermissionState::Allowed));
        assert_eq!(desktop.get("patch"), Some(&PermissionState::Allowed));
        assert_eq!(desktop.get("process"), Some(&PermissionState::Allowed));
    }

    // P19.9 — PermissionState serde roundtrip
    #[test]
    fn permission_state_serde() {
        let state = PermissionState::Allowed;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"allowed\"");
        let back: PermissionState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PermissionState::Allowed);

        let denied = PermissionState::Denied;
        let json = serde_json::to_string(&denied).unwrap();
        assert_eq!(json, "\"denied\"");
    }

    // P19.10 — ToolPermissions roundtrips through TOML
    #[test]
    fn tool_permissions_toml_roundtrip() {
        let config = ToolPermissions::default();
        let toml_str = toml::to_string(&config).unwrap();
        let back: ToolPermissions = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.low_risk_default, PermissionState::Allowed);
        assert_eq!(back.high_risk_default, PermissionState::Denied);
        assert!(back.overrides.contains_key("desktop"));
    }

    // Custom override on a channel surface
    #[test]
    fn custom_channel_override() {
        let mut config = ToolPermissions::default();
        // Deny memory on telegram specifically
        config
            .overrides
            .entry("telegram".into())
            .or_default()
            .insert("memory".into(), PermissionState::Denied);

        let state = PermissionResolver::resolve(&config, "memory", RiskLevel::Medium, "telegram");
        assert_eq!(state, PermissionState::Denied);

        // But memory is still allowed on slack (no override)
        let state = PermissionResolver::resolve(&config, "memory", RiskLevel::Medium, "slack");
        assert_eq!(state, PermissionState::Allowed);
    }
}
