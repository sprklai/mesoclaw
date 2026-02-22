use std::{collections::HashMap, sync::Arc};

use super::profiles::ToolProfile;
use super::traits::{Tool, ToolInfo};

/// Central registry of available tools.
///
/// Wrap in `Arc<Mutex<ToolRegistry>>` if registration must happen post-startup;
/// for a one-time setup at app launch, `Arc<ToolRegistry>` with a fully
/// populated registry is sufficient.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.  Overwrites any previous tool with the same name.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// List all registered tools (for inclusion in LLM `tools` arrays).
    pub fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|t| ToolInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                schema: t.parameters_schema(),
            })
            .collect()
    }

    /// List tools filtered by a tool profile.
    ///
    /// Returns only the tools that are allowed by the given profile.
    pub fn list_filtered(&self, profile: ToolProfile) -> Vec<ToolInfo> {
        self.tools
            .values()
            .filter(|t| profile.is_tool_allowed(t.name()))
            .map(|t| ToolInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                schema: t.parameters_schema(),
            })
            .collect()
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Iterate over all registered tools (unfiltered).
    ///
    /// Returns an iterator yielding `(name, tool)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<dyn Tool>)> {
        self.tools.iter()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::{Value, json};

    use crate::tools::traits::ToolResult;

    struct DummyTool(&'static str);

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            "A test tool"
        }
        fn parameters_schema(&self) -> Value {
            json!({"type": "object"})
        }
        async fn execute(&self, _args: Value) -> Result<ToolResult, String> {
            Ok(ToolResult::ok("done"))
        }
    }

    #[test]
    fn register_and_get() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool("dummy")));
        assert!(reg.get("dummy").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn list_returns_all() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool("dummy")));
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "dummy");
    }

    #[test]
    fn overwrite_same_name() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool("dummy")));
        reg.register(Arc::new(DummyTool("dummy")));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn list_filtered_by_profile() {
        let mut reg = ToolRegistry::new();
        // Register tools from different groups
        reg.register(Arc::new(DummyTool("shell"))); // Runtime
        reg.register(Arc::new(DummyTool("file_read"))); // Fs
        reg.register(Arc::new(DummyTool("file_write"))); // Fs
        reg.register(Arc::new(DummyTool("memory_recall"))); // Memory
        reg.register(Arc::new(DummyTool("web_fetch"))); // Web
        reg.register(Arc::new(DummyTool("custom_tool"))); // Unknown - allowed by default

        // Minimal profile: Fs + Memory only
        let minimal_tools = reg.list_filtered(ToolProfile::Minimal);
        let minimal_names: Vec<&str> = minimal_tools.iter().map(|t| t.name.as_str()).collect();
        assert!(minimal_names.contains(&"file_read"));
        assert!(minimal_names.contains(&"memory_recall"));
        assert!(!minimal_names.contains(&"shell"));
        assert!(!minimal_names.contains(&"web_fetch"));
        assert!(minimal_names.contains(&"custom_tool")); // Unknown tools allowed

        // Coding profile: Runtime + Fs + Memory
        let coding_tools = reg.list_filtered(ToolProfile::Coding);
        let coding_names: Vec<&str> = coding_tools.iter().map(|t| t.name.as_str()).collect();
        assert!(coding_names.contains(&"shell"));
        assert!(coding_names.contains(&"file_read"));
        assert!(coding_names.contains(&"memory_recall"));
        assert!(!coding_names.contains(&"web_fetch"));

        // Full profile: all known tools
        let full_tools = reg.list_filtered(ToolProfile::Full);
        assert_eq!(full_tools.len(), 6);
    }
}
