use std::sync::Arc;

use dashmap::DashMap;

use crate::Result;
use crate::error::MesoError;
use crate::tools::traits::{Tool, ToolInfo};

/// Centralized tool registry with duplicate rejection and lock-free reads.
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
        }
    }

    /// Register a tool. Returns error if a tool with the same name already exists.
    /// Uses DashMap `entry()` API to avoid TOCTOU race between contains_key + insert.
    pub fn register(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let name = tool.name().to_string();
        match self.tools.entry(name.clone()) {
            dashmap::mapref::entry::Entry::Occupied(_) => {
                Err(MesoError::Tool(format!("tool already registered: {name}")))
            }
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(tool);
                Ok(())
            }
        }
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|r| Arc::clone(r.value()))
    }

    /// List all registered tools as ToolInfo.
    pub fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .iter()
            .map(|entry| {
                let tool = entry.value();
                ToolInfo {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters_schema(),
                    risk_level: tool.risk_level(),
                }
            })
            .collect()
    }

    /// Get all tools as a Vec for passing to agent builders.
    pub fn to_vec(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.iter().map(|r| Arc::clone(r.value())).collect()
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty.
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
    use serde_json::json;

    use crate::tools::traits::ToolResult;

    struct FakeTool {
        tool_name: String,
    }

    impl FakeTool {
        fn new(name: &str) -> Self {
            Self {
                tool_name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for FakeTool {
        fn name(&self) -> &str {
            &self.tool_name
        }
        fn description(&self) -> &str {
            "fake tool for testing"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            json!({"type": "object"})
        }
        async fn execute(&self, _args: serde_json::Value) -> crate::Result<ToolResult> {
            Ok(ToolResult::ok("fake output"))
        }
    }

    #[test]
    fn register_and_get_by_name() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(FakeTool::new("test_tool"));
        registry.register(tool).unwrap();
        let found = registry.get("test_tool");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "test_tool");
    }

    #[test]
    fn reject_duplicate_name() {
        let registry = ToolRegistry::new();
        registry
            .register(Arc::new(FakeTool::new("dup_tool")))
            .unwrap();
        let result = registry.register(Arc::new(FakeTool::new("dup_tool")));
        assert!(result.is_err());
    }

    #[test]
    fn list_all_registered_tools() {
        let registry = ToolRegistry::new();
        registry.register(Arc::new(FakeTool::new("alpha"))).unwrap();
        registry.register(Arc::new(FakeTool::new("beta"))).unwrap();
        let list = registry.list();
        assert_eq!(list.len(), 2);
        let names: Vec<&str> = list.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn get_unknown_returns_none() {
        let registry = ToolRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn concurrent_register_and_get() {
        use std::thread;
        let registry = Arc::new(ToolRegistry::new());
        let mut handles = vec![];
        for i in 0..10 {
            let reg = Arc::clone(&registry);
            handles.push(thread::spawn(move || {
                let tool = Arc::new(FakeTool::new(&format!("tool_{i}")));
                reg.register(tool).unwrap();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(registry.len(), 10);
        for i in 0..10 {
            assert!(registry.get(&format!("tool_{i}")).is_some());
        }
    }

    // WS-6.6 — Concurrent duplicate registration is atomic (no TOCTOU)
    #[test]
    fn concurrent_duplicate_rejected_atomically() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::thread;
        let registry = Arc::new(ToolRegistry::new());
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];
        // 10 threads all try to register the same name
        for _ in 0..10 {
            let reg = Arc::clone(&registry);
            let count = Arc::clone(&success_count);
            handles.push(thread::spawn(move || {
                let tool = Arc::new(FakeTool::new("race_tool"));
                if reg.register(tool).is_ok() {
                    count.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        // Exactly one should succeed
        assert_eq!(success_count.load(Ordering::SeqCst), 1);
        assert_eq!(registry.len(), 1);
    }
}
