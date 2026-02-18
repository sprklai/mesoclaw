use std::{collections::HashMap, sync::Arc};

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

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
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

    struct DummyTool;

    #[async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &str {
            "dummy"
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
        reg.register(Arc::new(DummyTool));
        assert!(reg.get("dummy").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn list_returns_all() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "dummy");
    }

    #[test]
    fn overwrite_same_name() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        reg.register(Arc::new(DummyTool));
        assert_eq!(reg.len(), 1);
    }
}
