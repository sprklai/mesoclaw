use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use rig::completion::ToolDefinition;
use rig::tool::{ToolDyn, ToolError};
use rig::wasm_compat::WasmBoxedFuture;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::tools::Tool;

/// Cached result from a tool call.
#[derive(Debug, Clone)]
pub struct CachedResult {
    /// Serialized output (JSON for success, error string for failure).
    pub output: String,
    /// Logical tool success (from ToolResult.success on Ok path, false on Err path).
    pub success: bool,
    /// Whether the original call() returned Ok (true) or Err (false).
    pub is_ok: bool,
}

/// Per-request cache for deduplicating identical tool calls.
///
/// Keyed by `hash(tool_name + canonical_args_json)`. Shared across all adapters
/// for a single request via `Arc`. Tracks actual execution count for
/// continuation strategy awareness. Supports per-tool call limits to cap
/// expensive tools (e.g. web_search) regardless of argument differences.
pub struct ToolCallCache {
    entries: DashMap<u64, CachedResult>,
    call_count: AtomicU32,
    /// Per-tool execution counts (tool_name → count).
    per_tool_counts: DashMap<String, u32>,
    /// Most recent result per tool (for returning when over limit).
    per_tool_last: DashMap<String, CachedResult>,
    /// Per-tool call caps. Only tools listed here are limited.
    tool_call_limits: HashMap<String, usize>,
}

impl Default for ToolCallCache {
    fn default() -> Self {
        Self {
            entries: DashMap::new(),
            call_count: AtomicU32::new(0),
            per_tool_counts: DashMap::new(),
            per_tool_last: DashMap::new(),
            tool_call_limits: HashMap::new(),
        }
    }
}

impl ToolCallCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cache with per-tool call limits.
    pub fn with_limits(limits: HashMap<String, usize>) -> Self {
        Self {
            tool_call_limits: limits,
            ..Self::default()
        }
    }

    /// Compute cache key from tool name and args JSON string.
    /// Canonicalizes JSON (sorted keys, normalized whitespace) before hashing.
    pub fn cache_key(tool_name: &str, args: &str) -> u64 {
        let canonical = serde_json::from_str::<serde_json::Value>(args)
            .ok()
            .and_then(|v| serde_json::to_string(&v).ok())
            .unwrap_or_else(|| args.to_string());
        let mut hasher = DefaultHasher::new();
        tool_name.hash(&mut hasher);
        canonical.hash(&mut hasher);
        hasher.finish()
    }

    /// Check if a result is cached.
    pub fn get(&self, key: u64) -> Option<CachedResult> {
        self.entries.get(&key).map(|r| r.clone())
    }

    /// Store a result in the cache.
    pub fn insert(&self, key: u64, result: CachedResult) {
        self.entries.insert(key, result);
    }

    /// Increment the actual execution counter (called on cache miss).
    pub fn record_execution(&self) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Number of actual (non-cached) tool executions.
    pub fn executions(&self) -> u32 {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Check if a tool has exceeded its per-tool call limit.
    /// Returns `Some(last_result)` if over limit, `None` if under or no limit set.
    pub fn check_per_tool_limit(&self, tool_name: &str) -> Option<CachedResult> {
        let limit = self.tool_call_limits.get(tool_name)?;
        let count = self
            .per_tool_counts
            .get(tool_name)
            .map(|r| *r as usize)
            .unwrap_or(0);
        if count >= *limit {
            self.per_tool_last.get(tool_name).map(|r| r.clone())
        } else {
            None
        }
    }

    /// Record a per-tool execution and store the latest result.
    pub fn record_per_tool(&self, tool_name: &str, result: CachedResult) {
        self.per_tool_counts
            .entry(tool_name.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);
        self.per_tool_last.insert(tool_name.to_string(), result);
    }
}

impl std::fmt::Debug for ToolCallCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolCallCache")
            .field("entries", &self.entries.len())
            .field("call_count", &self.call_count.load(Ordering::Relaxed))
            .field("tool_call_limits", &self.tool_call_limits)
            .finish()
    }
}

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
    #[serde(rename = "cached")]
    Cached { output: String, success: bool },
    #[serde(rename = "approval_requested")]
    ApprovalRequested {
        approval_id: String,
        reason: String,
        risk_level: String,
        timeout_secs: u64,
    },
    #[serde(rename = "approval_resolved")]
    ApprovalResolved {
        approval_id: String,
        decision: String,
    },
}

/// Bridges a Zenii `Tool` trait object to rig-core's `ToolDyn` trait,
/// allowing Zenii tools to be used with rig agents.
pub struct RigToolAdapter {
    tool: Arc<dyn Tool>,
    event_tx: Option<broadcast::Sender<ToolCallEvent>>,
    cache: Option<Arc<ToolCallCache>>,
    approval_broker: Option<Arc<crate::security::approval::ApprovalBroker>>,
    event_bus: Option<Arc<dyn crate::event_bus::EventBus>>,
    surface: String,
    approval_timeout_secs: u64,
}

impl RigToolAdapter {
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self {
            tool,
            event_tx: None,
            cache: None,
            approval_broker: None,
            event_bus: None,
            surface: "desktop".into(),
            approval_timeout_secs: 120,
        }
    }

    /// Create an adapter with an event sender for tool call visibility.
    pub fn new_with_events(tool: Arc<dyn Tool>, tx: broadcast::Sender<ToolCallEvent>) -> Self {
        Self {
            tool,
            event_tx: Some(tx),
            cache: None,
            approval_broker: None,
            event_bus: None,
            surface: "desktop".into(),
            approval_timeout_secs: 120,
        }
    }

    /// Attach an approval broker for interactive tool approval.
    pub fn with_approval(
        mut self,
        broker: Arc<crate::security::approval::ApprovalBroker>,
        event_bus: Arc<dyn crate::event_bus::EventBus>,
        surface: &str,
        timeout_secs: u64,
    ) -> Self {
        self.approval_broker = Some(broker);
        self.event_bus = Some(event_bus);
        self.surface = surface.to_string();
        self.approval_timeout_secs = timeout_secs;
        self
    }

    /// Attach a dedup cache to this adapter (builder pattern).
    pub fn with_cache(mut self, cache: Arc<ToolCallCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Convert a list of Zenii tools into boxed rig ToolDyn objects.
    pub fn from_tools(tools: &[Arc<dyn Tool>]) -> Vec<Box<dyn ToolDyn>> {
        tools
            .iter()
            .map(|t| Box::new(Self::new(Arc::clone(t))) as Box<dyn ToolDyn>)
            .collect()
    }

    /// Convert a list of Zenii tools into boxed rig ToolDyn objects with event broadcasting.
    pub fn from_tools_with_events(
        tools: &[Arc<dyn Tool>],
        tx: broadcast::Sender<ToolCallEvent>,
    ) -> Vec<Box<dyn ToolDyn>> {
        tools
            .iter()
            .map(|t| Box::new(Self::new_with_events(Arc::clone(t), tx.clone())) as Box<dyn ToolDyn>)
            .collect()
    }

    /// Convert tools with event broadcasting and a shared dedup cache.
    pub fn from_tools_with_events_and_cache(
        tools: &[Arc<dyn Tool>],
        tx: broadcast::Sender<ToolCallEvent>,
        cache: Arc<ToolCallCache>,
    ) -> Vec<Box<dyn ToolDyn>> {
        tools
            .iter()
            .map(|t| {
                Box::new(
                    Self::new_with_events(Arc::clone(t), tx.clone()).with_cache(Arc::clone(&cache)),
                ) as Box<dyn ToolDyn>
            })
            .collect()
    }

    /// Convert tools with a shared dedup cache (no event broadcasting).
    pub fn from_tools_with_cache(
        tools: &[Arc<dyn Tool>],
        cache: Arc<ToolCallCache>,
    ) -> Vec<Box<dyn ToolDyn>> {
        tools
            .iter()
            .map(|t| {
                Box::new(Self::new(Arc::clone(t)).with_cache(Arc::clone(&cache)))
                    as Box<dyn ToolDyn>
            })
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

            // Check cache first (before emitting Started)
            if let Some(ref cache) = self.cache {
                let key = ToolCallCache::cache_key(&tool_name, &args);
                // 1. Exact-match dedup (same tool + same canonical args)
                if let Some(cached) = cache.get(key) {
                    if let Some(ref tx) = self.event_tx {
                        let _ = tx.send(ToolCallEvent {
                            call_id,
                            tool_name,
                            phase: ToolCallPhase::Cached {
                                output: cached.output.clone(),
                                success: cached.success,
                            },
                        });
                    }
                    return if cached.is_ok {
                        Ok(cached.output)
                    } else {
                        Err(ToolError::ToolCallError(Box::new(std::io::Error::other(
                            cached.output,
                        ))))
                    };
                }

                // 2. Per-tool limit check (different args but same tool over limit)
                if let Some(last) = cache.check_per_tool_limit(&tool_name) {
                    if let Some(ref tx) = self.event_tx {
                        let _ = tx.send(ToolCallEvent {
                            call_id,
                            tool_name,
                            phase: ToolCallPhase::Cached {
                                output: last.output.clone(),
                                success: last.success,
                            },
                        });
                    }
                    return if last.is_ok {
                        Ok(last.output)
                    } else {
                        Err(ToolError::ToolCallError(Box::new(std::io::Error::other(
                            last.output,
                        ))))
                    };
                }
            }

            // Approval gate: check if this tool needs user approval
            if let Some(ref broker) = self.approval_broker
                && let Some(reason) = self.tool.needs_approval(&args_value)
            {
                let args_summary = args_value
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&args)
                    .to_string();
                let risk_level = format!("{:?}", self.tool.risk_level()).to_lowercase();

                // Check pre-approved (session cache or DB rule)
                let pre = broker
                    .pre_check(&tool_name, &args_summary, &self.surface)
                    .await;

                match pre {
                    Some(crate::security::approval::ApprovalDecision::Deny) => {
                        return Err(ToolError::ToolCallError(Box::new(std::io::Error::other(
                            format!("Tool '{tool_name}' denied by saved rule"),
                        ))));
                    }
                    Some(_) => {
                        // Pre-approved, continue to execution
                    }
                    None => {
                        // Need to prompt user
                        let approval_id = uuid::Uuid::new_v4().to_string();

                        // Emit approval requested event via tool events
                        if let Some(ref tx) = self.event_tx {
                            let _ = tx.send(ToolCallEvent {
                                call_id: call_id.clone(),
                                tool_name: tool_name.clone(),
                                phase: ToolCallPhase::ApprovalRequested {
                                    approval_id: approval_id.clone(),
                                    reason: reason.clone(),
                                    risk_level: risk_level.clone(),
                                    timeout_secs: self.approval_timeout_secs,
                                },
                            });
                        }

                        // Also publish to event bus for notifications WS
                        if let Some(ref bus) = self.event_bus {
                            let _ = bus.publish(crate::event_bus::AppEvent::ApprovalRequested {
                                approval_id: approval_id.clone(),
                                call_id: call_id.clone(),
                                tool_name: tool_name.clone(),
                                args_summary: args_summary.clone(),
                                risk_level: risk_level.clone(),
                                reason: reason.clone(),
                                timeout_secs: self.approval_timeout_secs,
                            });
                        }

                        // Wait for decision with timeout
                        let rx = broker.register(&approval_id);
                        let timeout = std::time::Duration::from_secs(self.approval_timeout_secs);
                        let decision = tokio::select! {
                            result = rx => {
                                result.unwrap_or(crate::security::approval::ApprovalDecision::Deny)
                            }
                            _ = tokio::time::sleep(timeout) => {
                                crate::security::approval::ApprovalDecision::Deny
                            }
                        };

                        // Emit resolution event
                        if let Some(ref tx) = self.event_tx {
                            let _ = tx.send(ToolCallEvent {
                                call_id: call_id.clone(),
                                tool_name: tool_name.clone(),
                                phase: ToolCallPhase::ApprovalResolved {
                                    approval_id: approval_id.clone(),
                                    decision: decision.as_str().to_string(),
                                },
                            });
                        }

                        match decision {
                            crate::security::approval::ApprovalDecision::Approve => {
                                broker.cache_session(&tool_name, decision);
                            }
                            crate::security::approval::ApprovalDecision::ApproveAlways => {
                                broker.cache_session(&tool_name, decision);
                                let _ = broker
                                    .save_rule(
                                        &tool_name,
                                        Some(&args_summary),
                                        decision,
                                        &self.surface,
                                    )
                                    .await;
                            }
                            crate::security::approval::ApprovalDecision::Deny => {
                                return Err(ToolError::ToolCallError(Box::new(
                                    std::io::Error::other(format!(
                                        "Tool '{tool_name}' denied by user"
                                    )),
                                )));
                            }
                        }
                    }
                }
            }

            // Emit Started event (cache miss)
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

                    // Store in cache and record execution
                    if let Some(ref cache) = self.cache {
                        let key = ToolCallCache::cache_key(&tool_name, &args);
                        let cached = CachedResult {
                            output: output.clone(),
                            success: result.success,
                            is_ok: true,
                        };
                        cache.insert(key, cached.clone());
                        cache.record_per_tool(&tool_name, cached);
                        cache.record_execution();
                    }

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
                    // Store error in cache and record execution
                    if let Some(ref cache) = self.cache {
                        let key = ToolCallCache::cache_key(&tool_name, &args);
                        let cached = CachedResult {
                            output: e.to_string(),
                            success: false,
                            is_ok: false,
                        };
                        cache.insert(key, cached.clone());
                        cache.record_per_tool(&tool_name, cached);
                        cache.record_execution();
                    }

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
            Err(crate::ZeniiError::Tool("tool failed".into()))
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

    // TC-D1 — Cache hit returns cached result
    #[tokio::test]
    async fn tc_d1_cache_hit_returns_cached() {
        let cache = Arc::new(ToolCallCache::new());
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(Arc::clone(&tool)).with_cache(Arc::clone(&cache));

        let r1 = adapter
            .call(json!({"input": "hello"}).to_string())
            .await
            .unwrap();
        let r2 = adapter
            .call(json!({"input": "hello"}).to_string())
            .await
            .unwrap();
        assert_eq!(r1, r2);
    }

    // TC-D2 — Cache miss returns None
    #[test]
    fn tc_d2_cache_miss_returns_none() {
        let cache = ToolCallCache::new();
        let key = ToolCallCache::cache_key("test", r#"{"input":"hi"}"#);
        assert!(cache.get(key).is_none());
    }

    // TC-D3 — Error results are cached
    #[tokio::test]
    async fn tc_d3_error_results_are_cached() {
        let cache = Arc::new(ToolCallCache::new());
        let tool: Arc<dyn Tool> = Arc::new(FailingTool);
        let adapter = RigToolAdapter::new(tool).with_cache(Arc::clone(&cache));

        assert!(adapter.call("{}".to_string()).await.is_err());
        assert!(adapter.call("{}".to_string()).await.is_err());
        assert_eq!(cache.executions(), 1);
    }

    // TC-D4 — Cache disabled (no cache attached)
    #[tokio::test]
    async fn tc_d4_no_cache_no_dedup() {
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool);

        let r1 = adapter
            .call(json!({"input": "a"}).to_string())
            .await
            .unwrap();
        let r2 = adapter
            .call(json!({"input": "a"}).to_string())
            .await
            .unwrap();
        assert_eq!(r1, r2);
    }

    // TC-D5 — call_count increments on actual exec, not cache hit
    #[tokio::test]
    async fn tc_d5_call_count_tracks_actual_executions() {
        let cache = Arc::new(ToolCallCache::new());
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool).with_cache(Arc::clone(&cache));

        let _ = adapter.call(json!({"input": "hello"}).to_string()).await;
        assert_eq!(cache.executions(), 1);

        let _ = adapter.call(json!({"input": "hello"}).to_string()).await;
        assert_eq!(cache.executions(), 1); // cached — no increment

        let _ = adapter.call(json!({"input": "world"}).to_string()).await;
        assert_eq!(cache.executions(), 2);
    }

    // TC-D6 — Per-request scope: separate caches are independent
    #[tokio::test]
    async fn tc_d6_separate_caches_independent() {
        let cache1 = Arc::new(ToolCallCache::new());
        let cache2 = Arc::new(ToolCallCache::new());
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });

        let a1 = RigToolAdapter::new(Arc::clone(&tool)).with_cache(cache1.clone());
        let a2 = RigToolAdapter::new(Arc::clone(&tool)).with_cache(cache2.clone());

        let _ = a1.call(json!({"input": "hello"}).to_string()).await;
        assert_eq!(cache1.executions(), 1);
        assert_eq!(cache2.executions(), 0);

        let _ = a2.call(json!({"input": "hello"}).to_string()).await;
        assert_eq!(cache2.executions(), 1);
    }

    // TC-D7 — Cached event emitted on cache hit
    #[tokio::test]
    async fn tc_d7_cached_event_emitted() {
        let cache = Arc::new(ToolCallCache::new());
        let (tx, mut rx) = broadcast::channel::<ToolCallEvent>(16);
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new_with_events(tool, tx).with_cache(Arc::clone(&cache));

        let _ = adapter.call(json!({"input": "hi"}).to_string()).await;
        let _started = rx.recv().await.unwrap();
        let _completed = rx.recv().await.unwrap();

        let _ = adapter.call(json!({"input": "hi"}).to_string()).await;
        let cached_event = rx.recv().await.unwrap();
        assert!(matches!(
            cached_event.phase,
            ToolCallPhase::Cached { success: true, .. }
        ));
    }

    // TC-D8 — Different args produce different cache keys
    #[tokio::test]
    async fn tc_d8_different_args_different_keys() {
        let cache = Arc::new(ToolCallCache::new());
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool).with_cache(Arc::clone(&cache));

        let _ = adapter.call(json!({"input": "a"}).to_string()).await;
        let _ = adapter.call(json!({"input": "b"}).to_string()).await;
        assert_eq!(cache.executions(), 2);
    }

    // TC-D9 — Shared cache across adapters: same tool+args hits
    #[tokio::test]
    async fn tc_d9_shared_cache_across_adapters() {
        let cache = Arc::new(ToolCallCache::new());
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });

        let a1 = RigToolAdapter::new(Arc::clone(&tool)).with_cache(Arc::clone(&cache));
        let a2 = RigToolAdapter::new(Arc::clone(&tool)).with_cache(Arc::clone(&cache));

        let _ = a1.call(json!({"input": "hello"}).to_string()).await;
        assert_eq!(cache.executions(), 1);

        let _ = a2.call(json!({"input": "hello"}).to_string()).await;
        assert_eq!(cache.executions(), 1); // cache hit
    }

    // TC-D10 — Adapter without cache works normally (backwards compat)
    #[tokio::test]
    async fn tc_d10_adapter_without_cache_backwards_compat() {
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool);
        let result = adapter
            .call(json!({"input": "hello"}).to_string())
            .await
            .unwrap();
        let parsed: ToolResult = serde_json::from_str(&result).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.output, "processed: hello");
    }

    // =========================================================================
    // Per-Tool Limit Tests (TC-PL*)
    // =========================================================================

    // TC-PL1 — Per-tool limit blocks second call to same tool with different args
    #[tokio::test]
    async fn tc_pl1_per_tool_limit_blocks_different_args() {
        use std::collections::HashMap;
        let limits = HashMap::from([("test".to_string(), 1)]);
        let cache = Arc::new(ToolCallCache::with_limits(limits));
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool).with_cache(Arc::clone(&cache));

        let r1 = adapter
            .call(json!({"input": "alpha"}).to_string())
            .await
            .unwrap();
        let r2 = adapter
            .call(json!({"input": "beta"}).to_string())
            .await
            .unwrap();

        // Second call should return same result as first (per-tool limit hit)
        assert_eq!(r1, r2);
        assert_eq!(cache.executions(), 1);
    }

    // TC-PL2 — Per-tool limit returns last result (not first or error)
    #[tokio::test]
    async fn tc_pl2_per_tool_limit_returns_last_result() {
        use std::collections::HashMap;
        let limits = HashMap::from([("test".to_string(), 2)]);
        let cache = Arc::new(ToolCallCache::with_limits(limits));
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool).with_cache(Arc::clone(&cache));

        let _ = adapter
            .call(json!({"input": "first"}).to_string())
            .await
            .unwrap();
        let r2 = adapter
            .call(json!({"input": "second"}).to_string())
            .await
            .unwrap();
        let r3 = adapter
            .call(json!({"input": "third"}).to_string())
            .await
            .unwrap();

        // r3 should be the same as r2 (last executed result before limit hit)
        assert_eq!(r2, r3);
        assert_eq!(cache.executions(), 2);
    }

    // TC-PL3 — Per-tool limit does not affect unlisted tools
    #[tokio::test]
    async fn tc_pl3_per_tool_limit_unlisted_tools_unaffected() {
        use std::collections::HashMap;
        let limits = HashMap::from([("web_search".to_string(), 1)]);
        let cache = Arc::new(ToolCallCache::with_limits(limits));
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new(tool).with_cache(Arc::clone(&cache));

        let _ = adapter.call(json!({"input": "a"}).to_string()).await;
        let _ = adapter.call(json!({"input": "b"}).to_string()).await;
        let _ = adapter.call(json!({"input": "c"}).to_string()).await;

        // "test" is not in limits, so all 3 calls should execute
        assert_eq!(cache.executions(), 3);
    }

    // TC-PL4 — Per-tool limit emits Cached event when blocking
    #[tokio::test]
    async fn tc_pl4_per_tool_limit_emits_cached_event() {
        use std::collections::HashMap;
        let limits = HashMap::from([("test".to_string(), 1)]);
        let cache = Arc::new(ToolCallCache::with_limits(limits));
        let (tx, mut rx) = broadcast::channel::<ToolCallEvent>(16);
        let tool: Arc<dyn Tool> = Arc::new(MockTool { name: "test" });
        let adapter = RigToolAdapter::new_with_events(tool, tx).with_cache(Arc::clone(&cache));

        let _ = adapter.call(json!({"input": "a"}).to_string()).await;
        let _started = rx.recv().await.unwrap();
        let _completed = rx.recv().await.unwrap();

        // Second call with different args — should be blocked by per-tool limit
        let _ = adapter.call(json!({"input": "b"}).to_string()).await;
        let event = rx.recv().await.unwrap();
        assert!(matches!(event.phase, ToolCallPhase::Cached { .. }));
    }

    // TC-PL5 — check_per_tool_limit returns None when under limit
    #[test]
    fn tc_pl5_check_per_tool_limit_under() {
        use std::collections::HashMap;
        let limits = HashMap::from([("web_search".to_string(), 2)]);
        let cache = ToolCallCache::with_limits(limits);

        assert!(cache.check_per_tool_limit("web_search").is_none());

        cache.record_per_tool(
            "web_search",
            CachedResult {
                output: "ok".into(),
                success: true,
                is_ok: true,
            },
        );
        assert!(cache.check_per_tool_limit("web_search").is_none()); // 1 < 2
    }

    // TC-PL6 — record_per_tool increments count correctly
    #[test]
    fn tc_pl6_record_per_tool_increments() {
        use std::collections::HashMap;
        let limits = HashMap::from([("web_search".to_string(), 2)]);
        let cache = ToolCallCache::with_limits(limits);

        let result = CachedResult {
            output: "r1".into(),
            success: true,
            is_ok: true,
        };
        cache.record_per_tool("web_search", result);
        assert!(cache.check_per_tool_limit("web_search").is_none()); // 1 < 2

        let result2 = CachedResult {
            output: "r2".into(),
            success: true,
            is_ok: true,
        };
        cache.record_per_tool("web_search", result2);
        let blocked = cache.check_per_tool_limit("web_search");
        assert!(blocked.is_some()); // 2 >= 2
        assert_eq!(blocked.unwrap().output, "r2"); // returns last result
    }

    // =========================================================================
    // Canonicalization Tests (TC-CK*)
    // =========================================================================

    // TC-CK1 — Same JSON with different key order → same cache key
    #[test]
    fn tc_ck1_different_key_order_same_cache_key() {
        let k1 = ToolCallCache::cache_key("web_search", r#"{"query":"rust","num_results":5}"#);
        let k2 = ToolCallCache::cache_key("web_search", r#"{"num_results":5,"query":"rust"}"#);
        assert_eq!(k1, k2);
    }

    // TC-CK2 — Same JSON with different whitespace → same cache key
    #[test]
    fn tc_ck2_different_whitespace_same_cache_key() {
        let k1 = ToolCallCache::cache_key("web_search", r#"{"query":"rust"}"#);
        let k2 = ToolCallCache::cache_key("web_search", r#"{ "query" : "rust" }"#);
        assert_eq!(k1, k2);
    }
}
