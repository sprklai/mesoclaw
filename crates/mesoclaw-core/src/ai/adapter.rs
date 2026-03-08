use std::sync::Arc;
use std::time::Instant;

use rig::completion::ToolDefinition;
use rig::tool::{ToolDyn, ToolError};
use rig::wasm_compat::WasmBoxedFuture;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::tools::Tool;

/// Event emitted by a tool adapter during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEvent {
    pub call_id: String,
    pub tool_name: String,
    pub phase: ToolCallPhase,
}

/// Phase of a tool call lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "phase")]
pub enum ToolCallPhase {
    #[serde(rename = "started")]
    Started { args: serde_json::Value },
    #[serde(rename = "completed")]
    Completed {
        output: String,
        success: bool,
        duration_ms: u64,
    },
}

/// Bridges a MesoClaw `Tool` trait object to rig-core's `ToolDyn` trait,
/// allowing MesoClaw tools to be used with rig agents.
pub struct RigToolAdapter {
    tool: Arc<dyn Tool>,
    event_tx: Option<broadcast::Sender<ToolCallEvent>>,
}

impl RigToolAdapter {
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self {
            tool,
            event_tx: None,
        }
    }

    /// Create an adapter with an event sender for tool call visibility.
    pub fn new_with_events(tool: Arc<dyn Tool>, tx: broadcast::Sender<ToolCallEvent>) -> Self {
        Self {
            tool,
            event_tx: Some(tx),
        }
    }

    /// Convert a list of MesoClaw tools into boxed rig ToolDyn objects.
    pub fn from_tools(tools: &[Arc<dyn Tool>]) -> Vec<Box<dyn ToolDyn>> {
        tools
            .iter()
            .map(|t| Box::new(Self::new(Arc::clone(t))) as Box<dyn ToolDyn>)
            .collect()
    }

    /// Convert a list of MesoClaw tools into boxed rig ToolDyn objects with event broadcasting.
    pub fn from_tools_with_events(
        tools: &[Arc<dyn Tool>],
        tx: broadcast::Sender<ToolCallEvent>,
    ) -> Vec<Box<dyn ToolDyn>> {
        tools
            .iter()
            .map(|t| Box::new(Self::new_with_events(Arc::clone(t), tx.clone())) as Box<dyn ToolDyn>)
            .collect()
    }
}

impl ToolDyn for RigToolAdapter {
    fn name(&self) -> String {
        self.tool.name().to_string()
    }

    fn definition<'a>(&'a self, _prompt: String) -> WasmBoxedFuture<'a, ToolDefinition> {
        Box::pin(async move {
            ToolDefinition {
                name: self.tool.name().to_string(),
                description: self.tool.description().to_string(),
                parameters: self.tool.parameters_schema(),
            }
        })
    }

    fn call<'a>(&'a self, args: String) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        Box::pin(async move {
            let args_value: serde_json::Value =
                serde_json::from_str(&args).map_err(ToolError::JsonError)?;

            let call_id = uuid::Uuid::new_v4().to_string();
            let tool_name = self.tool.name().to_string();

            // Emit Started event
            if let Some(ref tx) = self.event_tx {
                let _ = tx.send(ToolCallEvent {
                    call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    phase: ToolCallPhase::Started {
                        args: args_value.clone(),
                    },
                });
            }

            let start = Instant::now();
            let exec_result = self.tool.execute(args_value).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            match exec_result {
                Ok(result) => {
                    let output = serde_json::to_string(&result).map_err(ToolError::JsonError)?;

                    // Emit Completed event
                    if let Some(ref tx) = self.event_tx {
                        let _ = tx.send(ToolCallEvent {
                            call_id,
                            tool_name,
                            phase: ToolCallPhase::Completed {
                                output: output.clone(),
                                success: result.success,
                                duration_ms,
                            },
                        });
                    }

                    Ok(output)
                }
                Err(e) => {
                    // Emit Completed with failure
                    if let Some(ref tx) = self.event_tx {
                        let _ = tx.send(ToolCallEvent {
                            call_id,
                            tool_name,
                            phase: ToolCallPhase::Completed {
                                output: e.to_string(),
                                success: false,
                                duration_ms,
                            },
                        });
                    }

                    Err(ToolError::ToolCallError(Box::new(e)))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolResult;
    use async_trait::async_trait;
    use serde_json::json;

    struct MockTool {
        name: &'static str,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            "A mock tool for testing"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            })
        }
        async fn execute(&self, args: serde_json::Value) -> crate::Result<ToolResult> {
            let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("none");
            Ok(ToolResult::ok(format!("processed: {input}")))
        }
    }

    struct FailingTool;

    #[async_trait]
    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "failing"
        }
        fn description(&self) -> &str {
            "Always fails"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            json!({})
        }
        async fn execute(&self, _args: serde_json::Value) -> crate::Result<ToolResult> {
            Err(crate::MesoError::Tool("tool failed".into()))
        }
    }

    // 1.1.1 — adapter name matches tool
    #[test]
    fn adapter_name_matches_tool() {
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test_tool" });
        let adapter = RigToolAdapter::new(tool);
        assert_eq!(ToolDyn::name(&adapter), "test_tool");
    }

    // 1.1.2 — adapter definition matches schema
    #[tokio::test]
    async fn adapter_definition_matches_schema() {
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test_tool" });
        let adapter = RigToolAdapter::new(tool);
        let def = adapter.definition("".to_string()).await;

        assert_eq!(def.name, "test_tool");
        assert_eq!(def.description, "A mock tool for testing");
        assert!(def.parameters.get("properties").is_some());
    }

    // 1.1.3 — adapter call delegates to tool
    #[tokio::test]
    async fn adapter_call_delegates_to_tool() {
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test_tool" });
        let adapter = RigToolAdapter::new(tool);
        let result = adapter
            .call(json!({"input": "hello"}).to_string())
            .await
            .unwrap();

        let parsed: ToolResult = serde_json::from_str(&result).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.output, "processed: hello");
    }

    // 1.1.4 — adapter call error propagates
    #[tokio::test]
    async fn adapter_call_error_propagates() {
        let tool: Arc<dyn Tool> = Arc::new(FailingTool);
        let adapter = RigToolAdapter::new(tool);
        let result = adapter.call("{}".to_string()).await;

        assert!(result.is_err());
    }

    // 1.1.5 — adapter from multiple tools
    #[test]
    fn adapter_from_multiple_tools() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool { name: "tool_a" }),
            Arc::new(MockTool { name: "tool_b" }),
        ];
        let rig_tools = RigToolAdapter::from_tools(&tools);

        assert_eq!(rig_tools.len(), 2);
        assert_eq!(rig_tools[0].name(), "tool_a");
        assert_eq!(rig_tools[1].name(), "tool_b");
    }

    // TV.1 — ToolCallEvent serializes with call_id and tool_name
    #[test]
    fn tool_call_event_serializes() {
        let event = ToolCallEvent {
            call_id: "abc-123".into(),
            tool_name: "WebSearch".into(),
            phase: ToolCallPhase::Started {
                args: json!({"query": "rust"}),
            },
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["call_id"], "abc-123");
        assert_eq!(json["tool_name"], "WebSearch");
        assert_eq!(json["phase"]["phase"], "started");
        assert_eq!(json["phase"]["args"]["query"], "rust");
    }

    // TV.2 — RigToolAdapter with event sender emits Started on call
    #[tokio::test]
    async fn adapter_emits_started_event() {
        let (tx, mut rx) = broadcast::channel::<ToolCallEvent>(8);
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new_with_events(tool, tx);

        let _ = adapter.call(json!({"input": "hi"}).to_string()).await;

        let event = rx.recv().await.unwrap();
        assert_eq!(event.tool_name, "test");
        assert!(matches!(event.phase, ToolCallPhase::Started { .. }));
    }

    // TV.3 — RigToolAdapter with event sender emits Completed on success
    #[tokio::test]
    async fn adapter_emits_completed_on_success() {
        let (tx, mut rx) = broadcast::channel::<ToolCallEvent>(8);
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new_with_events(tool, tx);

        let _ = adapter.call(json!({"input": "hi"}).to_string()).await;

        let _started = rx.recv().await.unwrap();
        let completed = rx.recv().await.unwrap();
        assert!(matches!(
            completed.phase,
            ToolCallPhase::Completed { success: true, .. }
        ));
    }

    // TV.4 — RigToolAdapter with event sender emits Completed with success=false on error
    #[tokio::test]
    async fn adapter_emits_completed_on_error() {
        let (tx, mut rx) = broadcast::channel::<ToolCallEvent>(8);
        let tool: Arc<dyn Tool> = Arc::new(FailingTool);
        let adapter = RigToolAdapter::new_with_events(tool, tx);

        let _ = adapter.call("{}".to_string()).await;

        let _started = rx.recv().await.unwrap();
        let completed = rx.recv().await.unwrap();
        assert!(matches!(
            completed.phase,
            ToolCallPhase::Completed { success: false, .. }
        ));
    }

    // TV.5 — RigToolAdapter without event sender works normally (backwards compat)
    #[tokio::test]
    async fn adapter_without_events_works() {
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool);
        let result = adapter
            .call(json!({"input": "hello"}).to_string())
            .await
            .unwrap();

        let parsed: ToolResult = serde_json::from_str(&result).unwrap();
        assert!(parsed.success);
    }

    // TV.6 — from_tools_with_events clones sender to all adapters
    #[tokio::test]
    async fn from_tools_with_events_clones_sender() {
        let (tx, mut rx) = broadcast::channel::<ToolCallEvent>(16);
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool { name: "tool_a" }),
            Arc::new(MockTool { name: "tool_b" }),
        ];
        let adapters = RigToolAdapter::from_tools_with_events(&tools, tx);

        assert_eq!(adapters.len(), 2);

        // Call both adapters — both should emit events
        let _ = adapters[0].call(json!({"input": "a"}).to_string()).await;
        let _ = adapters[1].call(json!({"input": "b"}).to_string()).await;

        // 4 events total: 2 Started + 2 Completed
        let mut events = vec![];
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }
        assert_eq!(events.len(), 4);
    }

    // TV.7 — ToolCallEvent includes duration_ms in Completed phase
    #[tokio::test]
    async fn completed_event_has_duration() {
        let (tx, mut rx) = broadcast::channel::<ToolCallEvent>(8);
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new_with_events(tool, tx);

        let _ = adapter.call(json!({"input": "hi"}).to_string()).await;

        let _started = rx.recv().await.unwrap();
        let completed = rx.recv().await.unwrap();
        if let ToolCallPhase::Completed { duration_ms, .. } = completed.phase {
            // Duration should be non-negative (it's u64, so always >= 0)
            assert!(duration_ms < 10_000); // sanity check: less than 10s
        } else {
            panic!("expected Completed phase");
        }
    }
}
