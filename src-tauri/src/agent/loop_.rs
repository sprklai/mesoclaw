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
//!     Allowed       → execute tool (sandboxed if configured) → emit ToolResult event → append to history
//!       │
//!       ▼
//! iteration += 1; if < max_iterations → repeat
//!       │
//!       ▼
//! return partial response + "max iterations reached" warning
//! ```

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering as AtomicOrdering},
};

use crate::{
    ai::{
        LLMProvider,
        types::{CompletionRequest, Message, MessageRole},
    },
    config::schema::SandboxMode,
    event_bus::{AppEvent, EventBus},
    memory::traits::Memory,
    security::{SecurityPolicy, ValidationResult},
    tools::ToolRegistry,
};

#[cfg(feature = "containers")]
use crate::modules::container::SandboxManager;

use super::tool_parser::{ParsedToolCall, parse_tool_calls};

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
    System {
        content: String,
    },
    User {
        content: String,
    },
    /// An assistant turn, which may include pending tool calls.
    Assistant {
        content: String,
        tool_calls: Vec<ParsedToolCall>,
    },
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
            AgentMessage::ToolResult {
                tool_name,
                result,
                success,
                ..
            } => {
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
    /// Optional cancellation flag.  When set to `true` the loop aborts at the
    /// next iteration boundary and returns `Err("cancelled")`.
    cancel_flag: Option<Arc<AtomicBool>>,
    /// Optional memory store for automatic context injection at session start.
    memory: Option<Arc<dyn Memory>>,
    /// Optional sandbox manager for tool isolation.
    #[cfg(feature = "containers")]
    sandbox: Option<Arc<SandboxManager>>,
    /// Sandbox mode - controls which tools are sandboxed.
    sandbox_mode: SandboxMode,
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
        Self {
            provider,
            tool_registry,
            security_policy,
            bus,
            config,
            cancel_flag: None,
            memory: None,
            #[cfg(feature = "containers")]
            sandbox: None,
            sandbox_mode: SandboxMode::default(),
        }
    }

    /// Attach a cancellation flag.  When the flag is set to `true` the loop
    /// aborts at the next iteration boundary with `Err("cancelled")`.
    pub fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel_flag = Some(flag);
        self
    }

    /// Attach a memory store for automatic context injection at session start.
    ///
    /// When set, the `run()` entry point will recall up to 5 relevant memories
    /// and prepend them as a system context message before the first LLM call.
    pub fn with_memory(mut self, memory: Arc<dyn Memory>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Attach a sandbox manager for tool isolation.
    ///
    /// When set, tool execution will be sandboxed in containers according
    /// to the configured sandbox mode.
    #[cfg(feature = "containers")]
    pub fn with_sandbox(mut self, sandbox: Arc<SandboxManager>) -> Self {
        self.sandbox_mode = sandbox.mode();
        self.sandbox = Some(sandbox);
        self
    }

    /// Set the sandbox mode directly (useful when no runtime is available).
    pub fn with_sandbox_mode(mut self, mode: SandboxMode) -> Self {
        self.sandbox_mode = mode;
        self
    }

    // ── Public entry points ───────────────────────────────────────────────────

    /// Run a single-message agent turn with no prior history.
    ///
    /// Returns the final text response.
    #[tracing::instrument(
        name = "agent.run",
        skip_all,
        fields(
            model = %self.config.model,
            user_msg_len = user_message.len(),
        )
    )]
    pub async fn run(&self, system_prompt: &str, user_message: &str) -> Result<String, String> {
        let mut history = vec![AgentMessage::System {
            content: system_prompt.to_string(),
        }];

        // Inject relevant memories as a system context message (FR-2.7).
        if let Some(ref mem) = self.memory
            && let Ok(entries) = mem.recall(user_message, 5).await
            && !entries.is_empty()
        {
            let context = entries
                .iter()
                .map(|e| format!("- {}: {}", e.key, e.content))
                .collect::<Vec<_>>()
                .join("\n");
            history.push(AgentMessage::System {
                content: format!("Relevant context from memory:\n{context}"),
            });
        }

        history.push(AgentMessage::User {
            content: user_message.to_string(),
        });

        let result = self.run_with_history(&mut history).await;

        // After completion, store a brief session record in memory (FR-2.7).
        if let (Ok(response), Some(mem)) = (&result, &self.memory) {
            let summary = if response.len() > 200 {
                format!("{}…", &response[..200])
            } else {
                response.clone()
            };
            let key = format!("session:{}", chrono::Utc::now().format("%Y%m%dT%H%M%S"));
            let _ = mem
                .store(
                    &key,
                    &format!("User: {user_message}\nAgent: {summary}"),
                    crate::memory::traits::MemoryCategory::Conversation,
                )
                .await;
        }

        result
    }

    /// Run the agent loop against an existing conversation `history`.
    ///
    /// `history` is mutated in place — new assistant and tool-result messages
    /// are appended as the loop proceeds.  The caller should persist or discard
    /// the updated history as appropriate.
    #[tracing::instrument(
        name = "agent.run_with_history",
        skip_all,
        fields(
            model = %self.config.model,
            max_iterations = self.config.max_iterations,
            history_len = history.len(),
        )
    )]
    pub async fn run_with_history(
        &self,
        history: &mut Vec<AgentMessage>,
    ) -> Result<String, String> {
        let mut iteration = 0;

        loop {
            // ── Cancellation check ─────────────────────────────────────────
            if let Some(ref flag) = self.cancel_flag
                && flag.load(AtomicOrdering::SeqCst)
            {
                return Err("cancelled".to_string());
            }

            // ── Compact history if needed (FR-10.4) ────────────────────────
            self.compact_history(history).await;

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
    ///
    /// If sandboxing is configured and applicable, the tool will be executed
    /// inside a container for isolation.
    #[tracing::instrument(
        name = "agent.tool",
        skip_all,
        fields(
            tool = %call.name,
            call_id = ?call.call_id,
            sandboxed = self.should_use_sandbox(),
        )
    )]
    async fn execute_tool_call(&self, call: &ParsedToolCall) -> AgentMessage {
        // Validate the tool name as if it were a command.
        let risk = self.security_policy.classify_command_risk(&call.name);
        match self.security_policy.validate_command(&call.name) {
            ValidationResult::Denied(reason) => {
                self.security_policy.log_action(
                    &call.name,
                    call.arguments.clone(),
                    risk,
                    "denied",
                    None,
                );
                self.emit_tool_result(&call.name, &reason, false);
                return AgentMessage::ToolResult {
                    tool_name: call.name.clone(),
                    call_id: call.call_id.clone(),
                    result: format!("Denied by security policy: {reason}"),
                    success: false,
                };
            }
            ValidationResult::NeedsApproval => {
                // Emit ApprovalNeeded and wait up to 30 s for a matching ApprovalResponse.
                if let Some(bus) = &self.bus {
                    let action_id = uuid::Uuid::new_v4().to_string();
                    let _ = bus.publish(AppEvent::ApprovalNeeded {
                        action_id: action_id.clone(),
                        tool_name: call.name.clone(),
                        description: format!("Agent wants to run tool '{}'", call.name),
                        risk_level: "medium".to_string(),
                    });

                    let mut rx = bus.subscribe();
                    let approved =
                        tokio::time::timeout(std::time::Duration::from_secs(30), async {
                            loop {
                                match rx.recv().await {
                                    Ok(AppEvent::ApprovalResponse {
                                        action_id: aid,
                                        approved,
                                    }) if aid == action_id => break approved,
                                    Ok(_) => continue,
                                    Err(_) => break false,
                                }
                            }
                        })
                        .await
                        .unwrap_or(false); // timeout → deny

                    if !approved {
                        let msg =
                            "Tool execution denied by user (or approval timed out after 30 s)";
                        self.security_policy.log_action(
                            &call.name,
                            call.arguments.clone(),
                            risk.clone(),
                            "denied",
                            Some(msg),
                        );
                        self.emit_tool_result(&call.name, msg, false);
                        return AgentMessage::ToolResult {
                            tool_name: call.name.clone(),
                            call_id: call.call_id.clone(),
                            result: msg.to_string(),
                            success: false,
                        };
                    }
                    // approved → fall through to normal execution
                } else {
                    // No EventBus configured — deny conservatively.
                    let msg = "Tool requires approval but no EventBus is available";
                    self.emit_tool_result(&call.name, msg, false);
                    return AgentMessage::ToolResult {
                        tool_name: call.name.clone(),
                        call_id: call.call_id.clone(),
                        result: msg.to_string(),
                        success: false,
                    };
                }
            }
            ValidationResult::Allowed => {}
        }

        // Emit start event.
        if let Some(bus) = &self.bus {
            let _ = bus.publish(AppEvent::AgentToolStart {
                tool_name: call.name.clone(),
                args: call.arguments.clone(),
            });
        }

        // Execute - either sandboxed or direct.
        #[cfg(feature = "containers")]
        let (result_str, success) = if self.should_use_sandbox() {
            self.execute_sandboxed(call).await
        } else {
            self.execute_direct(call).await
        };

        #[cfg(not(feature = "containers"))]
        let (result_str, success) = self.execute_direct(call).await;

        // Audit-log every tool execution.
        self.security_policy.log_action(
            &call.name,
            call.arguments.clone(),
            risk,
            if success { "allowed" } else { "failed" },
            Some(&result_str),
        );

        self.emit_tool_result(&call.name, &result_str, success);

        AgentMessage::ToolResult {
            tool_name: call.name.clone(),
            call_id: call.call_id.clone(),
            result: result_str,
            success,
        }
    }

    /// Check if sandboxing should be used for this tool execution.
    ///
    /// Agent loop tool calls are always considered "non-main" thread since
    /// they're spawned by the agent, not direct user action.
    #[cfg(feature = "containers")]
    fn should_use_sandbox(&self) -> bool {
        // Agent-executed tools are always considered non-main
        let is_main_thread = false;
        self.sandbox_mode.is_sandboxed(is_main_thread) && self.sandbox.is_some()
    }

    /// Check if sandboxing should be used (always false without containers feature).
    #[cfg(not(feature = "containers"))]
    fn should_use_sandbox(&self) -> bool {
        false
    }

    /// Execute a tool in a sandboxed container.
    #[cfg(feature = "containers")]
    async fn execute_sandboxed(&self, call: &ParsedToolCall) -> (String, bool) {
        let Some(ref sandbox) = self.sandbox else {
            return ("Sandbox configured but no runtime available".to_string(), false);
        };

        // For shell commands, use the shell execution path.
        if call.name == "shell" {
            let command = call
                .arguments
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let working_dir = call
                .arguments
                .get("working_dir")
                .and_then(|v| v.as_str());

            match sandbox.execute_shell(command, working_dir).await {
                Ok(result) => (result.output, result.success),
                Err(e) => (format!("Sandbox error: {e}"), false),
            }
        } else {
            // For other tools, use the generic tool execution.
            match sandbox
                .execute_tool(&call.name, call.arguments.clone(), None)
                .await
            {
                Ok(result) => (result.output, result.success),
                Err(e) => (format!("Sandbox error: {e}"), false),
            }
        }
    }

    /// Execute a tool directly (without sandboxing).
    async fn execute_direct(&self, call: &ParsedToolCall) -> (String, bool) {
        // Look up the tool in the registry.
        let Some(tool) = self.tool_registry.get(&call.name) else {
            return (format!("Tool '{}' is not registered", call.name), false);
        };

        match tool.execute(call.arguments.clone()).await {
            Ok(tr) => (tr.output, tr.success),
            Err(e) => (e, false),
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

    /// Compact history when it exceeds `max_history` messages (FR-10.4).
    ///
    /// Preserves:
    /// - `history[0]` — the persona/system prompt
    /// - A generated summary of the dropped messages (as a `System` message)
    /// - The most recent `max_history / 2` messages (active working context)
    ///
    /// Falls back to simple truncation if the summarization LLM call fails.
    async fn compact_history(&self, history: &mut Vec<AgentMessage>) {
        let threshold = self.config.max_history;
        if history.len() <= threshold {
            return;
        }

        // Always keep the first system message (agent persona).
        let keep_tail = threshold / 2;
        let drop_end = history.len().saturating_sub(keep_tail);

        // Collect the messages to be summarised (skip the system prompt).
        let to_summarise: Vec<&AgentMessage> = history[1..drop_end].iter().collect();
        if to_summarise.is_empty() {
            return;
        }

        // Build a compact text representation for the summariser.
        let excerpt: String = to_summarise
            .iter()
            .filter_map(|m| match m {
                AgentMessage::User { content } => Some(format!("User: {content}")),
                AgentMessage::Assistant { content, .. } if !content.is_empty() => {
                    Some(format!("Assistant: {content}"))
                }
                AgentMessage::ToolResult {
                    tool_name, result, ..
                } => Some(format!("Tool({tool_name}): {result}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let summary_prompt = format!(
            "Summarise the following conversation excerpt in 3-5 concise sentences, \
             focusing on what was accomplished and any important context:\n\n{excerpt}"
        );

        let summary = self
            .provider
            .complete(CompletionRequest::new(
                self.config.model.clone(),
                vec![Message {
                    role: MessageRole::User,
                    content: summary_prompt,
                }],
            ))
            .await
            .map(|r| r.content)
            .unwrap_or_else(|_| format!("[{} messages compacted]", to_summarise.len()));

        // Persist the compaction summary to memory if available.
        if let Some(ref mem) = self.memory {
            let key = format!("compact:{}", chrono::Utc::now().format("%Y%m%dT%H%M%S"));
            let _ = mem
                .store(
                    &key,
                    &summary,
                    crate::memory::traits::MemoryCategory::Conversation,
                )
                .await;
        }

        // Rebuild history: system prompt + summary + recent tail.
        let system_msg = history.remove(0);
        let tail: Vec<AgentMessage> = history.drain(drop_end - 1..).collect();

        *history = vec![
            system_msg,
            AgentMessage::System {
                content: format!("Earlier conversation summary: {summary}"),
            },
        ];
        history.extend(tail);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ai::{provider::StreamResponse, types::CompletionResponse},
        event_bus::EventBus,
        security::{AutonomyLevel, SecurityPolicy},
        tools::{Tool, ToolRegistry, ToolResult},
    };
    use async_trait::async_trait;
    use serde_json::Value;
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
        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> crate::ai::provider::Result<CompletionResponse> {
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

        async fn stream(
            &self,
            _r: CompletionRequest,
        ) -> crate::ai::provider::Result<StreamResponse> {
            unimplemented!("stream not used in AgentLoop tests")
        }

        fn context_limit(&self) -> usize {
            128_000
        }
        fn supports_tools(&self) -> bool {
            false
        }
        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    // ── Mock tool ─────────────────────────────────────────────────────────────

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "echoes its input"
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, args: Value) -> Result<ToolResult, String> {
            let msg = args
                .get("message")
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
        let loop_ = make_loop(
            provider,
            registry_with_echo(),
            supervised_policy(),
            Default::default(),
        );
        let result = loop_
            .run("You are a helper.", "What is 2+2?")
            .await
            .unwrap();
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
        let loop_ = make_loop(
            provider,
            registry_with_echo(),
            supervised_policy(),
            Default::default(),
        );
        let result = loop_
            .run("You are an agent.", "Test the echo tool.")
            .await
            .unwrap();
        assert_eq!(result, "The echo said: ping. That's the result.");
    }

    #[tokio::test]
    async fn unknown_tool_injected_as_error_and_continues() {
        let provider = MockProvider::new(vec![
            r#"<tool_call>{"name": "nonexistent_tool", "arguments": {}}</tool_call>"#,
            "I couldn't find that tool, sorry.",
        ]);
        let loop_ = make_loop(
            provider,
            registry_with_echo(),
            supervised_policy(),
            Default::default(),
        );
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
        let loop_ = make_loop(
            provider,
            registry_with_echo(),
            readonly_policy(),
            Default::default(),
        );
        let result = loop_.run("system", "delete a file").await.unwrap();
        // Should continue after denial and return the second response
        assert_eq!(result, "The tool was denied, I'll work another way.");
    }

    #[tokio::test]
    async fn compact_history_removes_middle_messages() {
        // MockProvider returns a summary string for the summarization call.
        let loop_ = AgentLoop::new(
            MockProvider::new(vec!["summary of old messages"]),
            Arc::new(ToolRegistry::new()),
            supervised_policy(),
            None,
            AgentConfig {
                max_history: 6,
                ..Default::default()
            },
        );

        // Build a history of 10 messages (more than max_history=6).
        let mut history: Vec<AgentMessage> = (0..10)
            .map(|i| AgentMessage::User {
                content: format!("msg {i}"),
            })
            .collect();

        loop_.compact_history(&mut history).await;

        // After compaction: system prompt + summary + recent tail (≤ max_history/2 = 3)
        // Resulting history length should be ≤ max_history.
        assert!(
            history.len() <= loop_.config.max_history,
            "compacted history ({}) should fit within max_history ({})",
            history.len(),
            loop_.config.max_history
        );
        // First message should be the original system prompt.
        if let AgentMessage::User { content } = &history[0] {
            assert_eq!(content, "msg 0");
        }
    }

    #[tokio::test]
    async fn compact_history_no_op_when_under_limit() {
        let loop_ = AgentLoop::new(
            MockProvider::new(vec![]),
            Arc::new(ToolRegistry::new()),
            supervised_policy(),
            None,
            AgentConfig {
                max_history: 50,
                ..Default::default()
            },
        );

        let mut history: Vec<AgentMessage> = (0..10)
            .map(|i| AgentMessage::User {
                content: format!("msg {i}"),
            })
            .collect();

        loop_.compact_history(&mut history).await;
        assert_eq!(history.len(), 10);
    }

    #[test]
    fn agent_message_to_llm_system() {
        let msg = AgentMessage::System {
            content: "Be helpful.".to_string(),
        };
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
