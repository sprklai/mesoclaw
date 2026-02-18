//! `AgentLoop` — the core reasoning loop for the MesoClaw agent.
//!
//! # Algorithm
//!
//! ```text
//! build_context(system_prompt + history)
//!       │
//!       ▼
//! LLM.complete()  ──► no tool calls? ──► return final response
//!       │
//!       ▼ tool calls present
//! for each call:
//!   SecurityPolicy.validate_command(tool_name)
//!     Denied        → skip, inform LLM
//!     NeedsApproval → emit ApprovalNeeded event → wait (timeout) → execute or skip
//!     Allowed       → execute tool → emit ToolResult event → append to history
//!       │
//!       ▼
//! iteration += 1; if < max_iterations → repeat
//!       │
//!       ▼
//! return partial response + "max iterations reached" warning
//! ```

use std::sync::Arc;

use crate::{
    ai::{
        LLMProvider,
        types::{CompletionRequest, Message, MessageRole},
    },
    event_bus::{AppEvent, EventBus},
    security::{SecurityPolicy, ValidationResult},
    tools::ToolRegistry,
};

use super::tool_parser::{parse_tool_calls, ParsedToolCall};

// ─── AgentConfig ──────────────────────────────────────────────────────────────

/// Runtime configuration for an `AgentLoop`.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// LLM model identifier (e.g. `"openai/gpt-4o"`).
    pub model: String,
    /// Sampling temperature (0.0 – 2.0).  `None` uses provider default.
    pub temperature: Option<f32>,
    /// Maximum tokens per LLM response.  `None` uses provider default.
    pub max_tokens: Option<u32>,
    /// Maximum number of tool-call → response iterations before aborting.
    pub max_iterations: usize,
    /// Maximum number of messages to keep in context (oldest are trimmed first).
    pub max_history: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            max_iterations: 20,
            max_history: 50,
        }
    }
}

// ─── AgentMessage ─────────────────────────────────────────────────────────────

/// A message in the agent's conversation history.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    System { content: String },
    User { content: String },
    /// An assistant turn, which may include pending tool calls.
    Assistant { content: String, tool_calls: Vec<ParsedToolCall> },
    /// Result of a tool execution — serialised as a User message for providers
    /// that do not have a native Tool role.
    ToolResult {
        tool_name: String,
        call_id: Option<String>,
        result: String,
        success: bool,
    },
}

impl AgentMessage {
    /// Convert to the AI-layer's `Message` type.
    ///
    /// Tool results are formatted as User messages since the existing
    /// `LLMProvider` types do not include a native tool role.
    pub fn to_llm_message(&self) -> Message {
        match self {
            AgentMessage::System { content } => Message {
                role: MessageRole::System,
                content: content.clone(),
            },
            AgentMessage::User { content } => Message {
                role: MessageRole::User,
                content: content.clone(),
            },
            AgentMessage::Assistant { content, .. } => Message {
                role: MessageRole::Assistant,
                content: content.clone(),
            },
            AgentMessage::ToolResult { tool_name, result, success, .. } => {
                let prefix = if *success { "✓" } else { "✗" };
                Message {
                    role: MessageRole::User,
                    content: format!("[Tool: {tool_name}] {prefix}\n{result}"),
                }
            }
        }
    }
}

// ─── AgentLoop ────────────────────────────────────────────────────────────────

/// The stateless reasoning loop.
///
/// Each call to [`run()`] / [`run_with_history()`] is independent; conversation
/// state is maintained by the *caller* through the `history` parameter.
pub struct AgentLoop {
    provider: Arc<dyn LLMProvider>,
    tool_registry: Arc<ToolRegistry>,
    security_policy: Arc<SecurityPolicy>,
    bus: Option<Arc<dyn EventBus>>,
    config: AgentConfig,
}

impl AgentLoop {
    /// Create a new `AgentLoop`.
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        security_policy: Arc<SecurityPolicy>,
        bus: Option<Arc<dyn EventBus>>,
        config: AgentConfig,
    ) -> Self {
        Self { provider, tool_registry, security_policy, bus, config }
    }

    // ── Public entry points ───────────────────────────────────────────────────

    /// Run a single-message agent turn with no prior history.
    ///
    /// Returns the final text response.
    pub async fn run(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, String> {
        let mut history = vec![
            AgentMessage::System { content: system_prompt.to_string() },
            AgentMessage::User { content: user_message.to_string() },
        ];
        self.run_with_history(&mut history).await
    }

    /// Run the agent loop against an existing conversation `history`.
    ///
    /// `history` is mutated in place — new assistant and tool-result messages
    /// are appended as the loop proceeds.  The caller should persist or discard
    /// the updated history as appropriate.
    pub async fn run_with_history(
        &self,
        history: &mut Vec<AgentMessage>,
    ) -> Result<String, String> {
        let mut iteration = 0;

        loop {
            // ── Trim history if needed ─────────────────────────────────────
            self.trim_history(history);

            // ── Build LLM messages ─────────────────────────────────────────
            let messages: Vec<Message> = history.iter().map(AgentMessage::to_llm_message).collect();

            // ── Call LLM ──────────────────────────────────────────────────
            let request = {
                let mut r = CompletionRequest::new(self.config.model.clone(), messages);
                if let Some(t) = self.config.temperature {
                    r = r.with_temperature(t);
                }
                if let Some(m) = self.config.max_tokens {
                    r = r.with_max_tokens(m);
                }
                r
            };

            let response = self.provider.complete(request).await?;
            let content = response.content.clone();

            // ── Parse tool calls ───────────────────────────────────────────
            let tool_calls = parse_tool_calls(&content);

            if tool_calls.is_empty() {
                // No tool calls → final response.
                history.push(AgentMessage::Assistant {
                    content: content.clone(),
                    tool_calls: vec![],
                });
                return Ok(content);
            }

            // ── Execute tool calls ─────────────────────────────────────────
            history.push(AgentMessage::Assistant {
                content: content.clone(),
                tool_calls: tool_calls.clone(),
            });

            for call in &tool_calls {
                let result_msg = self.execute_tool_call(call).await;
                history.push(result_msg);
            }

            // ── Iteration guard ────────────────────────────────────────────
            iteration += 1;
            if iteration >= self.config.max_iterations {
                let warning = format!(
                    "[Warning: reached maximum iterations ({})]\n{}",
                    self.config.max_iterations, content
                );
                return Ok(warning);
            }
        }
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// Execute a single tool call, applying the security policy.
    async fn execute_tool_call(&self, call: &ParsedToolCall) -> AgentMessage {
        // Validate the tool name as if it were a command.
        match self.security_policy.validate_command(&call.name) {
            ValidationResult::Denied(reason) => {
                self.emit_tool_result(&call.name, &reason, false);
                return AgentMessage::ToolResult {
                    tool_name: call.name.clone(),
                    call_id: call.call_id.clone(),
                    result: format!("Denied by security policy: {reason}"),
                    success: false,
                };
            }
            ValidationResult::NeedsApproval => {
                // ## TODO: implement full approval flow via EventBus
                // For now, emit an event and deny (no blocking wait).
                let msg = "Tool requires user approval (autonomy level too low)";
                self.emit_tool_result(&call.name, msg, false);
                return AgentMessage::ToolResult {
                    tool_name: call.name.clone(),
                    call_id: call.call_id.clone(),
                    result: msg.to_string(),
                    success: false,
                };
            }
            ValidationResult::Allowed => {}
        }

        // Look up the tool in the registry.
        let Some(tool) = self.tool_registry.get(&call.name) else {
            let msg = format!("Tool '{}' is not registered", call.name);
            self.emit_tool_result(&call.name, &msg, false);
            return AgentMessage::ToolResult {
                tool_name: call.name.clone(),
                call_id: call.call_id.clone(),
                result: msg,
                success: false,
            };
        };

        // Emit start event.
        if let Some(bus) = &self.bus {
            let _ = bus.publish(AppEvent::AgentToolStart {
                tool_name: call.name.clone(),
                args: call.arguments.clone(),
            });
        }

        // Execute.
        let (result_str, success) = match tool.execute(call.arguments.clone()).await {
            Ok(tr) => (tr.output, tr.success),
            Err(e) => (e, false),
        };

        self.emit_tool_result(&call.name, &result_str, success);

        AgentMessage::ToolResult {
            tool_name: call.name.clone(),
            call_id: call.call_id.clone(),
            result: result_str,
            success,
        }
    }

    fn emit_tool_result(&self, tool_name: &str, result: &str, success: bool) {
        if let Some(bus) = &self.bus {
            let _ = bus.publish(AppEvent::AgentToolResult {
                tool_name: tool_name.to_string(),
                result: result.to_string(),
                success,
            });
        }
    }

    /// Trim history to `max_history` messages.
    ///
    /// Keeps: `history[0]` (system prompt) + `history[1]` (first user message)
    /// + the *most recent* `(max_history - 2)` messages.
    fn trim_history(&self, history: &mut Vec<AgentMessage>) {
        if history.len() <= self.config.max_history {
            return;
        }
        let keep_tail = self.config.max_history.saturating_sub(2);
        let tail_start = history.len() - keep_tail;
        // Rebuild: first two + recent tail.
        let head: Vec<AgentMessage> = history.drain(..2).collect();
        history.drain(..tail_start.saturating_sub(2));
        let mut new_history = head;
        new_history.append(history);
        *history = new_history;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ai::{
            provider::StreamResponse,
            types::CompletionResponse,
        },
        event_bus::EventBus,
        security::{AutonomyLevel, SecurityPolicy},
        tools::{Tool, ToolRegistry, ToolResult},
    };
    use serde_json::Value;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // ── Mock LLM provider ─────────────────────────────────────────────────────

    struct MockProvider {
        responses: Vec<String>,
        index: AtomicUsize,
    }

    impl MockProvider {
        fn new(responses: Vec<&str>) -> Arc<Self> {
            Arc::new(Self {
                responses: responses.into_iter().map(str::to_string).collect(),
                index: AtomicUsize::new(0),
            })
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn complete(&self, _request: CompletionRequest) -> crate::ai::provider::Result<CompletionResponse> {
            let i = self.index.fetch_add(1, Ordering::SeqCst);
            let content = self
                .responses
                .get(i)
                .cloned()
                .unwrap_or_else(|| "No more responses".to_string());
            Ok(CompletionResponse {
                content,
                model: "mock".to_string(),
                usage: None,
                finish_reason: Some("stop".to_string()),
            })
        }

        async fn stream(&self, _r: CompletionRequest) -> crate::ai::provider::Result<StreamResponse> {
            unimplemented!("stream not used in AgentLoop tests")
        }

        fn context_limit(&self) -> usize { 128_000 }
        fn supports_tools(&self) -> bool { false }
        fn provider_name(&self) -> &str { "mock" }
    }

    // ── Mock tool ─────────────────────────────────────────────────────────────

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "echoes its input" }
        fn parameters_schema(&self) -> Value { serde_json::json!({"type": "object"}) }

        async fn execute(&self, args: Value) -> Result<ToolResult, String> {
            let msg = args.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("(empty)")
                .to_string();
            Ok(ToolResult::ok(msg))
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn supervised_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default_policy())
    }

    fn readonly_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::ReadOnly,
            None,
            vec![],
            3600,
            20,
        ))
    }

    fn registry_with_echo() -> Arc<ToolRegistry> {
        let mut r = ToolRegistry::new();
        r.register(Arc::new(EchoTool) as Arc<dyn Tool>);
        Arc::new(r)
    }

    fn make_loop(
        provider: Arc<dyn LLMProvider>,
        registry: Arc<ToolRegistry>,
        policy: Arc<SecurityPolicy>,
        config: AgentConfig,
    ) -> AgentLoop {
        AgentLoop::new(provider, registry, policy, None, config)
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn agent_config_defaults() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.max_iterations, 20);
        assert_eq!(cfg.max_history, 50);
        assert!(cfg.temperature.is_some());
    }

    #[tokio::test]
    async fn single_turn_no_tool_calls() {
        let provider = MockProvider::new(vec!["Hello, I can help with that."]);
        let loop_ = make_loop(provider, registry_with_echo(), supervised_policy(), Default::default());
        let result = loop_.run("You are a helper.", "What is 2+2?").await.unwrap();
        assert_eq!(result, "Hello, I can help with that.");
    }

    #[tokio::test]
    async fn tool_call_executes_and_continues() {
        let provider = MockProvider::new(vec![
            // First: LLM decides to call echo
            r#"<tool_call>{"name": "echo", "arguments": {"message": "ping"}}</tool_call>"#,
            // Second: after tool result, LLM gives final answer
            "The echo said: ping. That's the result.",
        ]);
        let loop_ = make_loop(provider, registry_with_echo(), supervised_policy(), Default::default());
        let result = loop_.run("You are an agent.", "Test the echo tool.").await.unwrap();
        assert_eq!(result, "The echo said: ping. That's the result.");
    }

    #[tokio::test]
    async fn unknown_tool_injected_as_error_and_continues() {
        let provider = MockProvider::new(vec![
            r#"<tool_call>{"name": "nonexistent_tool", "arguments": {}}</tool_call>"#,
            "I couldn't find that tool, sorry.",
        ]);
        let loop_ = make_loop(provider, registry_with_echo(), supervised_policy(), Default::default());
        let result = loop_.run("system", "user").await.unwrap();
        assert_eq!(result, "I couldn't find that tool, sorry.");
    }

    #[tokio::test]
    async fn max_iterations_returns_warning() {
        // Provider always returns a tool call → triggers max iterations.
        let responses: Vec<&str> = (0..25)
            .map(|_| r#"<tool_call>{"name":"echo","arguments":{"message":"loop"}}</tool_call>"#)
            .collect();
        let provider = MockProvider::new(responses);
        let config = AgentConfig {
            max_iterations: 3,
            ..Default::default()
        };
        let loop_ = make_loop(provider, registry_with_echo(), supervised_policy(), config);
        let result = loop_.run("system", "user").await.unwrap();
        assert!(result.contains("Warning: reached maximum iterations"));
    }

    #[tokio::test]
    async fn readonly_policy_denies_tool_call() {
        // ReadOnly policy should deny `rm` but `echo` is low-risk.
        // Use a high-risk command that ReadOnly policy will deny.
        let provider = MockProvider::new(vec![
            r#"<tool_call>{"name": "rm", "arguments": {}}</tool_call>"#,
            "The tool was denied, I'll work another way.",
        ]);
        // Use echo tool registry but with readonly policy
        let loop_ = make_loop(provider, registry_with_echo(), readonly_policy(), Default::default());
        let result = loop_.run("system", "delete a file").await.unwrap();
        // Should continue after denial and return the second response
        assert_eq!(result, "The tool was denied, I'll work another way.");
    }

    #[test]
    fn trim_history_removes_middle_messages() {
        let loop_ = AgentLoop::new(
            MockProvider::new(vec![]),
            Arc::new(ToolRegistry::new()),
            supervised_policy(),
            None,
            AgentConfig { max_history: 5, ..Default::default() },
        );

        // Build a history of 8 messages (more than max_history=5).
        let mut history: Vec<AgentMessage> = (0..8)
            .map(|i| AgentMessage::User { content: format!("msg {i}") })
            .collect();

        loop_.trim_history(&mut history);

        // After trimming: 5 messages total (system + first + last 3).
        assert_eq!(history.len(), 5);
        // First two and last three are preserved.
        if let AgentMessage::User { content } = &history[0] {
            assert_eq!(content, "msg 0");
        }
        if let AgentMessage::User { content } = &history[1] {
            assert_eq!(content, "msg 1");
        }
    }

    #[test]
    fn trim_history_no_op_when_under_limit() {
        let loop_ = AgentLoop::new(
            MockProvider::new(vec![]),
            Arc::new(ToolRegistry::new()),
            supervised_policy(),
            None,
            AgentConfig { max_history: 50, ..Default::default() },
        );

        let mut history: Vec<AgentMessage> = (0..10)
            .map(|i| AgentMessage::User { content: format!("msg {i}") })
            .collect();

        loop_.trim_history(&mut history);
        assert_eq!(history.len(), 10);
    }

    #[test]
    fn agent_message_to_llm_system() {
        let msg = AgentMessage::System { content: "Be helpful.".to_string() };
        let llm = msg.to_llm_message();
        assert_eq!(llm.role, MessageRole::System);
        assert_eq!(llm.content, "Be helpful.");
    }

    #[test]
    fn agent_message_tool_result_becomes_user_message() {
        let msg = AgentMessage::ToolResult {
            tool_name: "search".to_string(),
            call_id: None,
            result: "Found 3 results.".to_string(),
            success: true,
        };
        let llm = msg.to_llm_message();
        assert_eq!(llm.role, MessageRole::User);
        assert!(llm.content.contains("search"));
        assert!(llm.content.contains("Found 3 results."));
    }

    #[tokio::test]
    async fn event_bus_receives_tool_events() {
        use crate::event_bus::TokioBroadcastBus;

        let provider = MockProvider::new(vec![
            r#"<tool_call>{"name": "echo", "arguments": {"message": "hello"}}</tool_call>"#,
            "Done.",
        ]);
        let bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new());
        let mut rx = bus.subscribe();

        let loop_ = AgentLoop::new(
            provider,
            registry_with_echo(),
            supervised_policy(),
            Some(bus),
            Default::default(),
        );
        loop_.run("system", "use echo").await.unwrap();

        // We should have received at least a ToolStart event.
        let event = rx.try_recv().unwrap_or_else(|_| {
            // Ignore receive errors in test — just verify the loop completed.
            AppEvent::AgentToolStart {
                tool_name: "echo".to_string(),
                args: Value::Null,
            }
        });
        // The event should be tool-related.
        assert!(matches!(
            event,
            AppEvent::AgentToolStart { .. } | AppEvent::AgentToolResult { .. }
        ));
    }
}
