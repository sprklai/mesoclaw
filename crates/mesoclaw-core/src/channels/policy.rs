use std::sync::Arc;

use crate::config::AppConfig;
use crate::security::permissions::{PermissionResolver, PermissionState};
use crate::tools::ToolRegistry;
use crate::tools::traits::Tool;

/// Filter a list of tool names by a comma-separated policy string.
/// If `policy` is empty, returns all tools (no filtering).
pub fn filter_tools_by_policy<'a>(all_tools: &[&'a str], policy: &str) -> Vec<&'a str> {
    if policy.is_empty() {
        return all_tools.to_vec();
    }
    let allowed: Vec<&str> = policy.split(',').map(|s| s.trim()).collect();
    all_tools
        .iter()
        .filter(|t| allowed.contains(t))
        .copied()
        .collect()
}

/// Per-channel tool policy — delegates to PermissionResolver.
pub struct ChannelToolPolicy {
    config: Arc<AppConfig>,
}

impl ChannelToolPolicy {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Get allowed tools for a channel, using the permission system.
    pub fn allowed_tools(&self, channel_name: &str, registry: &ToolRegistry) -> Vec<Arc<dyn Tool>> {
        PermissionResolver::allowed_tools(&self.config.tool_permissions, channel_name, registry)
    }

    /// Get names of allowed tools for a channel (for system context injection).
    pub fn allowed_tool_names(&self, channel_name: &str, registry: &ToolRegistry) -> Vec<String> {
        PermissionResolver::allowed_tool_names(
            &self.config.tool_permissions,
            channel_name,
            registry,
        )
    }

    /// Check if a specific tool is allowed on a channel.
    pub fn is_tool_allowed(
        &self,
        tool_name: &str,
        channel_name: &str,
        registry: &ToolRegistry,
    ) -> bool {
        if let Some(tool) = registry.get(tool_name) {
            matches!(
                PermissionResolver::resolve(
                    &self.config.tool_permissions,
                    tool_name,
                    tool.risk_level(),
                    channel_name,
                ),
                PermissionState::Allowed
            )
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry() -> ToolRegistry {
        use crate::security::policy::SecurityPolicy;
        use crate::tools::system_info::SystemInfoTool;

        let reg = ToolRegistry::new();
        reg.register(Arc::new(SystemInfoTool::new())).unwrap();
        reg.register(Arc::new(crate::tools::shell::ShellTool::new(
            Arc::new(SecurityPolicy::default_policy()),
            30,
        )))
        .unwrap();
        reg
    }

    // P19.15 — ChannelToolPolicy.allowed_tools delegates to PermissionResolver
    // With defaults: system_info (Low=Allowed) passes, shell (High=Denied) blocked on channels
    #[test]
    fn policy_delegates_to_resolver() {
        let config = Arc::new(AppConfig::default());
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let tools = policy.allowed_tools("telegram", &registry);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "system_info");
    }

    // P19.16 — Desktop surface allows all tools (high-risk overridden)
    #[test]
    fn desktop_allows_all_tools() {
        let config = Arc::new(AppConfig::default());
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let tools = policy.allowed_tools("desktop", &registry);
        assert_eq!(tools.len(), 2);
    }

    // P19.16b — Channel-specific tool override works
    #[test]
    fn channel_specific_override() {
        use crate::security::permissions::{PermissionState, ToolPermissions};
        let mut perms = ToolPermissions::default();
        perms
            .overrides
            .entry("telegram".into())
            .or_default()
            .insert("shell".into(), PermissionState::Allowed);

        let config = Arc::new(AppConfig {
            tool_permissions: perms,
            ..Default::default()
        });
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let tools = policy.allowed_tools("telegram", &registry);
        // Both system_info (Low=Allowed) and shell (override=Allowed)
        assert_eq!(tools.len(), 2);
    }

    // allowed_tool_names returns correct list
    #[test]
    fn allowed_tool_names_returns_names() {
        let config = Arc::new(AppConfig::default());
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let names = policy.allowed_tool_names("telegram", &registry);
        assert_eq!(names, vec!["system_info"]);
    }

    // WS2.3a — filter_tools_by_policy filters to only allowed tools
    #[test]
    fn channel_tool_policy_filters_tools() {
        let all_tools = vec!["shell", "file_read", "web_search", "system_info"];
        let policy = "web_search,system_info";
        let allowed = filter_tools_by_policy(&all_tools, policy);
        assert_eq!(allowed, vec!["web_search", "system_info"]);
    }

    // WS2.3b — filter_tools_by_policy with empty string allows all
    #[test]
    fn channel_tool_policy_empty_allows_all() {
        let all_tools = vec!["shell", "file_read", "web_search"];
        let policy = "";
        let allowed = filter_tools_by_policy(&all_tools, policy);
        assert_eq!(allowed, all_tools);
    }
}
