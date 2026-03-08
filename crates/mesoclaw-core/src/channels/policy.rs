use std::sync::Arc;

use crate::config::AppConfig;
use crate::tools::ToolRegistry;
use crate::tools::traits::Tool;

/// Per-channel tool allowlist policy.
pub struct ChannelToolPolicy {
    config: Arc<AppConfig>,
}

impl ChannelToolPolicy {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Default safe tools for channel messages.
    pub fn default_allowlist() -> Vec<String> {
        vec!["web_search".into(), "system_info".into()]
    }

    /// Get allowed tools for a channel, filtered from the registry.
    pub fn allowed_tools(&self, channel_name: &str, registry: &ToolRegistry) -> Vec<Arc<dyn Tool>> {
        let allowlist = self.allowlist_for(channel_name);

        if allowlist.is_empty() {
            return vec![];
        }

        registry
            .to_vec()
            .into_iter()
            .filter(|tool| allowlist.iter().any(|name| name == tool.name()))
            .collect()
    }

    fn allowlist_for(&self, channel_name: &str) -> Vec<String> {
        // Check channel-specific config first
        if let Some(tools) = self.config.channel_tool_policy.get(channel_name) {
            return tools.clone();
        }

        // Fall back to "default" key
        if let Some(tools) = self.config.channel_tool_policy.get("default") {
            return tools.clone();
        }

        // Fall back to hardcoded default
        Self::default_allowlist()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_config(policy: HashMap<String, Vec<String>>) -> Arc<AppConfig> {
        Arc::new(AppConfig {
            channel_tool_policy: policy,
            ..Default::default()
        })
    }

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

    // CR.8 — default_allowlist returns web_search and system_info
    #[test]
    fn default_allowlist_contents() {
        let list = ChannelToolPolicy::default_allowlist();
        assert!(list.contains(&"web_search".to_string()));
        assert!(list.contains(&"system_info".to_string()));
        assert_eq!(list.len(), 2);
    }

    // CR.9 — allowed_tools filters registry to only allowed tools
    #[test]
    fn filters_to_allowed_only() {
        let config = make_config(HashMap::from([(
            "default".into(),
            vec!["system_info".into()],
        )]));
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let tools = policy.allowed_tools("telegram", &registry);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "system_info");
    }

    // CR.10 — allowed_tools uses channel-specific config when present
    #[test]
    fn uses_channel_specific_config() {
        let config = make_config(HashMap::from([
            ("default".into(), vec!["system_info".into()]),
            (
                "telegram".into(),
                vec!["system_info".into(), "shell".into()],
            ),
        ]));
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let tools = policy.allowed_tools("telegram", &registry);
        assert_eq!(tools.len(), 2);
    }

    // CR.11 — allowed_tools falls back to default when channel not in config
    #[test]
    fn falls_back_to_default() {
        let config = make_config(HashMap::from([(
            "default".into(),
            vec!["system_info".into()],
        )]));
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let tools = policy.allowed_tools("discord", &registry);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "system_info");
    }

    // CR.12 — empty allowlist returns no tools
    #[test]
    fn empty_allowlist_no_tools() {
        let config = make_config(HashMap::from([("telegram".into(), vec![])]));
        let policy = ChannelToolPolicy::new(config);
        let registry = make_registry();

        let tools = policy.allowed_tools("telegram", &registry);
        assert!(tools.is_empty());
    }
}
