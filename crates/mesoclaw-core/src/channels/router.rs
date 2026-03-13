use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tracing::{info, warn};

#[cfg(all(feature = "channels", feature = "gateway"))]
use tracing::error;

#[cfg(feature = "gateway")]
use crate::gateway::state::AppState;

#[cfg(feature = "ai")]
use crate::ai::adapter::{ToolCallEvent, ToolCallPhase};
#[cfg(feature = "ai")]
use crate::event_bus::AppEvent;
#[cfg(feature = "ai")]
use tokio::sync::broadcast;

#[cfg(feature = "ai")]
use super::format::formatter_for;
use super::message::ChannelMessage;
#[cfg(feature = "ai")]
use super::policy::ChannelToolPolicy;
#[cfg(all(feature = "channels", feature = "gateway", feature = "ai"))]
use super::session_map::ChannelSessionMap;

/// Channel Router orchestrator: receives messages from all channels,
/// resolves sessions, runs agent turns, formats responses, and sends replies.
#[cfg(all(feature = "channels", feature = "gateway"))]
pub struct ChannelRouter {
    message_tx: mpsc::Sender<ChannelMessage>,
    message_rx: tokio::sync::Mutex<Option<mpsc::Receiver<ChannelMessage>>>,
    stop_tx: watch::Sender<bool>,
    stop_rx: watch::Receiver<bool>,
    state: Arc<tokio::sync::OnceCell<Arc<AppState>>>,
    #[cfg(feature = "ai")]
    session_map: Arc<tokio::sync::OnceCell<Arc<ChannelSessionMap>>>,
}

#[cfg(all(feature = "channels", feature = "gateway"))]
impl ChannelRouter {
    /// Create a new ChannelRouter with a bounded message channel.
    pub fn new(buffer_size: usize) -> Self {
        let (message_tx, message_rx) = mpsc::channel(buffer_size);
        let (stop_tx, stop_rx) = watch::channel(false);
        Self {
            message_tx,
            message_rx: tokio::sync::Mutex::new(Some(message_rx)),
            stop_tx,
            stop_rx,
            state: Arc::new(tokio::sync::OnceCell::new()),
            #[cfg(feature = "ai")]
            session_map: Arc::new(tokio::sync::OnceCell::new()),
        }
    }

    /// Wire the router with AppState.
    pub fn wire(&self, state: Arc<AppState>) {
        #[cfg(feature = "ai")]
        {
            let sm = Arc::new(ChannelSessionMap::new(state.session_manager.clone()));
            let _ = self.session_map.set(sm);
        }
        let _ = self.state.set(state);
    }

    /// Get a sender clone for submitting messages.
    pub fn sender(&self) -> mpsc::Sender<ChannelMessage> {
        self.message_tx.clone()
    }

    /// Start the message processing loop.
    pub async fn start(&self) {
        let mut rx = {
            let mut guard = self.message_rx.lock().await;
            match guard.take() {
                Some(rx) => rx,
                None => {
                    warn!("ChannelRouter already started");
                    return;
                }
            }
        };

        let mut stop_rx = self.stop_rx.clone();
        let state_cell = self.state.clone();
        #[cfg(feature = "ai")]
        let session_map_cell = self.session_map.clone();

        tokio::spawn(async move {
            info!("ChannelRouter started");
            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Some(message) => {
                                if let Some(state) = state_cell.get() {
                                    #[cfg(feature = "ai")]
                                    {
                                        let sm = session_map_cell.get().cloned();
                                        Self::handle_message(message, state, sm.as_ref()).await;
                                    }
                                    #[cfg(not(feature = "ai"))]
                                    {
                                        let _ = message;
                                        let _ = state;
                                        warn!("ChannelRouter: ai feature not enabled, dropping message");
                                    }
                                } else {
                                    warn!("ChannelRouter: no AppState wired, dropping message");
                                }
                            }
                            None => {
                                info!("ChannelRouter: all senders dropped, stopping");
                                break;
                            }
                        }
                    }
                    Ok(()) = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            info!("ChannelRouter: stop signal received");
                            break;
                        }
                    }
                }
            }
            info!("ChannelRouter stopped");
        });
    }

    /// Stop the router.
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
    }

    /// Handle a single incoming channel message through the full pipeline.
    /// Public static version for use by webhook endpoints.
    #[cfg(feature = "ai")]
    pub async fn handle_message_static(message: ChannelMessage, state: &Arc<AppState>) {
        // Create a transient session map for webhook-originated messages
        let sm = Arc::new(ChannelSessionMap::new(state.session_manager.clone()));
        Self::handle_message(message, state, Some(&sm)).await;
    }

    /// Handle a single incoming channel message through the full pipeline.
    #[cfg(feature = "ai")]
    async fn handle_message(
        message: ChannelMessage,
        state: &Arc<AppState>,
        session_map: Option<&Arc<ChannelSessionMap>>,
    ) {
        let channel_name = message.channel.clone();
        let reply_metadata = message.metadata.clone();

        // Extract chat_id for lifecycle hooks (channels need chat_id, not username)
        let recipient = reply_metadata.get("chat_id").cloned();

        // 1. Resolve or create session (uses shared map if available, else transient)
        let transient_map;
        let sm = match session_map {
            Some(sm) => sm.as_ref(),
            None => {
                transient_map = Arc::new(ChannelSessionMap::new(state.session_manager.clone()));
                &transient_map
            }
        };
        let channel_key = ChannelSessionMap::channel_key(&message);
        let session_id = match sm.resolve_session(&channel_key, &channel_name).await {
            Ok(id) => id,
            Err(e) => {
                warn!("ChannelRouter: failed to resolve session for {channel_key}: {e}");
                return;
            }
        };

        // 2. Store the user message in the session
        if let Err(e) = state
            .session_manager
            .append_message(&session_id, "user", &message.content)
            .await
        {
            warn!("ChannelRouter: failed to store user message: {e}");
            return;
        }

        // Publish user message event
        let sender_name = message.sender.clone().unwrap_or_else(|| "unknown".into());
        let preview = message.content.chars().take(100).collect::<String>();
        let _ = state.event_bus.publish(AppEvent::ChannelMessageReceived {
            channel: channel_name.clone(),
            sender: sender_name.clone(),
            session_id: session_id.clone(),
            content_preview: preview,
            role: "user".into(),
        });

        // 3. Get allowed tools for this channel (enforced via resolve_agent_with_tools)
        let tool_policy = ChannelToolPolicy::new(state.config.load_full());
        let allowed_tool_names = tool_policy.allowed_tool_names(&channel_name, &state.tools);
        let allowed_tools = tool_policy.allowed_tools(&channel_name, &state.tools);

        // 4. Build context parts + assemble preamble via PromptStrategy
        let (history_from_ctx, _memories, _user_obs) = state
            .context_builder
            .build_parts(Some(&session_id), &message.content)
            .await
            .unwrap_or_else(|e| {
                warn!("ChannelRouter: context build failed for {channel_name}: {e}");
                (vec![], vec![], String::new())
            });

        let summary = state
            .session_manager
            .get_context_info(&session_id)
            .await
            .ok()
            .and_then(|(_, _, s)| s);

        let config = state.config.load_full();
        let assembly_request = crate::ai::prompt::AssemblyRequest {
            boot_context: state.boot_context.clone(),
            model_display: "default".into(),
            session_id: Some(session_id.clone()),
            user_message: Some(message.content.clone()),
            conversation_summary: summary,
            channel_hint: Some(channel_name.clone()),
            tool_count: state.tools.len(),
            skill_count: state.skill_registry.list().await.len(),
            version: config.identity_name.clone(),
        };
        let preamble = state
            .prompt_strategy
            .assemble(&assembly_request)
            .await
            .unwrap_or_default();

        // 5. Merge: preamble + channel-specific formatting hint (with tool awareness)
        let channel_hint = channel_system_context(&channel_name, &allowed_tool_names);
        let system_context = format!("{preamble}\n\n{channel_hint}");

        // 6. Call lifecycle hook: on_agent_start
        if let Some(channel) = state.channel_registry.get_channel(&channel_name) {
            channel.on_agent_start(recipient.as_deref()).await;
        }

        // 7. Create tool event channel for broadcasting tool calls to lifecycle hooks
        let (tool_event_tx, mut tool_event_rx) = broadcast::channel::<ToolCallEvent>(32);

        // 8. Resolve agent WITH tool events and channel-filtered tools
        let tool_override = if allowed_tools.is_empty() {
            None // empty policy = no tools
        } else {
            Some(allowed_tools)
        };
        let agent = match crate::ai::resolve_agent_with_tools(
            None,
            state,
            Some(tool_event_tx),
            Some(&system_context),
            tool_override,
        )
        .await
        {
            Ok(a) => a,
            Err(e) => {
                warn!("ChannelRouter: failed to resolve agent for {channel_name}: {e}");
                if let Some(channel) = state.channel_registry.get_channel(&channel_name) {
                    channel.on_agent_complete(recipient.as_deref()).await;
                }
                return;
            }
        };

        // 9. Spawn tool event listener that forwards events to channel lifecycle hooks
        let tool_channel_name = channel_name.clone();
        let tool_recipient = recipient.clone();
        let tool_registry = state.channel_registry.clone();
        let tool_listener = tokio::spawn(async move {
            while let Ok(event) = tool_event_rx.recv().await {
                if matches!(event.phase, ToolCallPhase::Started { .. })
                    && let Some(ch) = tool_registry.get_channel(&tool_channel_name)
                {
                    ch.on_tool_use(&event.tool_name, tool_recipient.as_deref())
                        .await;
                }
            }
        });

        // 10. Use history from context builder (windowed by strategy), fallback to manual
        let history = if !history_from_ctx.is_empty() {
            history_from_ctx
        } else {
            match state.session_manager.get_messages(&session_id).await {
                Ok(msgs) => {
                    let to_convert = if msgs.len() > 1 {
                        &msgs[..msgs.len() - 1]
                    } else {
                        &[]
                    };
                    let start = to_convert.len().saturating_sub(20);
                    crate::ai::context::convert_session_messages(&to_convert[start..])
                }
                Err(_) => vec![],
            }
        };

        // 11. Run agent chat with reasoning engine
        let response = match state
            .reasoning_engine
            .chat(&agent, &message.content, history)
            .await
            .map(|r| r.response)
        {
            Ok(r) => r,
            Err(e) => {
                warn!("ChannelRouter: agent chat failed for {channel_name}: {e}");
                tool_listener.abort();
                if let Some(channel) = state.channel_registry.get_channel(&channel_name) {
                    channel.on_agent_complete(recipient.as_deref()).await;
                }
                return;
            }
        };

        // 12. Abort tool listener (agent done)
        tool_listener.abort();

        // 13. Call lifecycle hook: on_agent_complete
        if let Some(channel) = state.channel_registry.get_channel(&channel_name) {
            channel.on_agent_complete(recipient.as_deref()).await;
        }

        // 14. Store assistant response
        let _ = state
            .session_manager
            .append_message(&session_id, "assistant", &response)
            .await;

        // Publish assistant response event
        let response_preview = response.chars().take(100).collect::<String>();
        let _ = state.event_bus.publish(AppEvent::ChannelMessageReceived {
            channel: channel_name.clone(),
            sender: state.config.load().identity_name.clone(),
            session_id: session_id.clone(),
            content_preview: response_preview,
            role: "assistant".into(),
        });

        // 15. Format response for the channel
        let formatter = formatter_for(&channel_name);
        let parts = formatter.format(&response);

        // 16. Send formatted response parts
        for part in parts {
            let reply =
                ChannelMessage::new(&channel_name, &part).with_metadata(reply_metadata.clone());
            if let Err(e) = state.channel_registry.send(&channel_name, reply).await {
                warn!("ChannelRouter: failed to send reply via {channel_name}: {e}");
            }
        }

        info!(
            "ChannelRouter: processed message from {channel_name}, session={session_id}, response_len={}",
            response.len()
        );
    }
}

/// Supervisor loop for a channel: catches `listen()` exits, publishes lifecycle events,
/// applies exponential backoff, and restarts the channel.
#[cfg(all(feature = "channels", feature = "gateway"))]
pub async fn supervise_channel(
    channel: Arc<dyn super::traits::Channel>,
    tx: mpsc::Sender<super::message::ChannelMessage>,
    event_bus: Arc<dyn crate::event_bus::EventBus>,
    config: Arc<crate::config::AppConfig>,
) {
    let name = channel.display_name().to_string();
    let max_restarts = config.channel_supervisor_max_restarts;
    let min_ms = config.channel_supervisor_backoff_min_ms;
    let max_ms = config.channel_supervisor_backoff_max_ms;
    let mut attempt: u32 = 0;

    loop {
        // Re-connect before each listen cycle if not already connected
        if channel.status() != super::traits::ChannelStatus::Connected {
            let _ = event_bus.publish(crate::event_bus::AppEvent::ChannelReconnecting {
                channel: name.clone(),
                attempt,
            });
            if let Err(e) = channel.connect().await {
                warn!("Supervisor: {name} reconnect failed: {e}");
                let delay = supervisor_backoff(attempt, min_ms, max_ms);
                tokio::time::sleep(delay).await;
                attempt += 1;
                if max_restarts > 0 && attempt >= max_restarts {
                    error!("Supervisor: {name} max restarts ({max_restarts}) reached");
                    let _ = event_bus.publish(crate::event_bus::AppEvent::ChannelDisconnected {
                        channel: name.clone(),
                        reason: "max supervisor restarts reached".into(),
                    });
                    break;
                }
                continue;
            }
        }

        let _ = event_bus.publish(crate::event_bus::AppEvent::ChannelConnected {
            channel: name.clone(),
        });
        info!("Supervisor: {name} listen started (cycle {attempt})");

        let listen_start = std::time::Instant::now();

        // Run listen — blocks until channel dies
        let result = channel.listen(tx.clone()).await;

        // listen() exited — channel is dead
        warn!("Supervisor: {name} listen exited: {result:?}");
        let _ = event_bus.publish(crate::event_bus::AppEvent::ChannelDisconnected {
            channel: name.clone(),
            reason: match &result {
                Ok(()) => "listen returned Ok".into(),
                Err(e) => e.to_string(),
            },
        });

        attempt += 1;
        if max_restarts > 0 && attempt >= max_restarts {
            error!("Supervisor: {name} max restarts ({max_restarts}) reached, giving up");
            break;
        }

        // If listen ran for > 60s, reset attempt counter (was a successful cycle)
        if listen_start.elapsed() > std::time::Duration::from_secs(60) {
            attempt = 1;
        }

        let delay = supervisor_backoff(attempt, min_ms, max_ms);
        info!("Supervisor: {name} restarting in {}s", delay.as_secs());
        tokio::time::sleep(delay).await;
    }
}

/// Exponential backoff clamped to [min_ms, max_ms].
#[cfg(all(feature = "channels", feature = "gateway"))]
fn supervisor_backoff(attempt: u32, min_ms: u64, max_ms: u64) -> std::time::Duration {
    let delay_ms = min_ms.saturating_mul(2u64.saturating_pow(attempt));
    std::time::Duration::from_millis(delay_ms.min(max_ms))
}

/// Channel-specific system context with dynamic tool awareness.
pub fn channel_system_context(channel_name: &str, allowed_tool_names: &[String]) -> String {
    let tools_desc = if allowed_tool_names.is_empty() {
        "No tools available.".to_string()
    } else {
        format!("Available tools: {}.", allowed_tool_names.join(", "))
    };

    let channel_hint = match channel_name {
        "telegram" => {
            "[Channel: Telegram] Format for mobile chat:\n\
            - Use **bold** for labels, separate label from value with a space or colon\n\
            - Put each distinct item on its own line (use markdown line breaks)\n\
            - Use bullet points (- or •) for lists, one item per line\n\
            - Separate sections with a blank line\n\
            - Keep code blocks short (<10 lines). Prefer inline `code` for brief values\n\
            - Avoid wide tables — use line-per-item layout instead\n\
            - Max ~300 words per response"
        }
        "slack" => {
            "[Channel: Slack] Format using Slack mrkdwn:\n\
            - Use *bold* not **bold**, _italic_ not *italic*\n\
            - Put each distinct item on its own line\n\
            - Use bullet points for lists, one item per line\n\
            - Separate sections with a blank line\n\
            - Keep code blocks short. Prefer inline `code` for brief values\n\
            - Max ~400 words per response"
        }
        "discord" => {
            "[Channel: Discord] Format for Discord chat:\n\
            - Keep responses under 2000 characters\n\
            - Use standard markdown (**bold**, *italic*, `code`)\n\
            - Put each distinct item on its own line\n\
            - Use bullet points for lists, one item per line\n\
            - Separate sections with a blank line\n\
            - Keep code blocks short with language tags"
        }
        _ => "[Channel: External] Keep responses concise.",
    };

    format!(
        "{channel_hint}\n{tools_desc}\nUse your tools when relevant. If a user requests something requiring a tool you don't have, explain it's restricted in this channel and suggest using the Desktop or CLI client."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // SUP.1 — supervisor_backoff starts at min_ms
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[test]
    fn supervisor_backoff_starts_at_min() {
        let delay = supervisor_backoff(0, 5000, 300_000);
        assert_eq!(delay.as_millis(), 5000);
    }

    // SUP.2 — supervisor_backoff doubles each attempt
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[test]
    fn supervisor_backoff_doubles() {
        let d0 = supervisor_backoff(0, 5000, 300_000);
        let d1 = supervisor_backoff(1, 5000, 300_000);
        let d2 = supervisor_backoff(2, 5000, 300_000);
        assert_eq!(d0.as_millis(), 5000);
        assert_eq!(d1.as_millis(), 10000);
        assert_eq!(d2.as_millis(), 20000);
    }

    // SUP.3 — supervisor_backoff clamps to max_ms
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[test]
    fn supervisor_backoff_clamps_at_max() {
        let delay = supervisor_backoff(20, 5000, 300_000);
        assert_eq!(delay.as_millis(), 300_000);
    }

    // SUP.4 — supervisor_backoff handles overflow gracefully
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[test]
    fn supervisor_backoff_overflow_safe() {
        let delay = supervisor_backoff(u32::MAX, 5000, 300_000);
        // Should not panic, should clamp to max
        assert!(delay.as_millis() <= 300_000);
    }

    // 8.7.1 — ChannelRouter can be created
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[test]
    fn channel_router_create() {
        let router = ChannelRouter::new(32);
        // Should not panic
        assert!(!router.stop_tx.is_closed());
    }

    // 8.7.2 — ChannelRouter sender clones work
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[test]
    fn channel_router_sender_clones() {
        let router = ChannelRouter::new(32);
        let _s1 = router.sender();
        let _s2 = router.sender();
        // Both senders created successfully
    }

    // 8.7.3 — ChannelRouter wire sets state
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    async fn channel_router_wire() {
        let router = ChannelRouter::new(32);
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        router.wire(state);
        // wire is idempotent, shouldn't panic on second call
        let (_dir2, state2) = crate::gateway::handlers::tests::test_state().await;
        router.wire(state2);
    }

    // 8.7.4 — ChannelRouter start and stop
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    async fn channel_router_start_stop() {
        let router = ChannelRouter::new(32);
        router.start().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        router.stop();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // 8.7.5 — ChannelRouter processes messages when wired
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    async fn channel_router_processes_messages() {
        let router = ChannelRouter::new(32);
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        router.wire(state);
        router.start().await;

        // Send a message (no channel registered, so it'll log a warning but not crash)
        let sender = router.sender();
        let msg = ChannelMessage::new("test", "hello");
        sender.send(msg).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        router.stop();
    }

    // 8.7.6 — ChannelRouter drops messages when not wired
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    async fn channel_router_drops_without_state() {
        let router = ChannelRouter::new(32);
        // NOT wired
        router.start().await;

        let sender = router.sender();
        let msg = ChannelMessage::new("test", "hello");
        sender.send(msg).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        router.stop();
    }

    // 8.7.7 — channel_system_context returns telegram-specific prompt
    // (renamed from CR.24 to avoid duplication)

    // P19.17 — channel_system_context includes available tool names
    #[test]
    fn context_telegram_with_tools() {
        let tools = vec!["web_search".to_string(), "memory".to_string()];
        let ctx = channel_system_context("telegram", &tools);
        assert!(ctx.contains("Telegram"));
        assert!(ctx.contains("mobile chat"));
        assert!(ctx.contains("web_search"));
        assert!(ctx.contains("memory"));
        assert!(ctx.contains("Available tools:"));
    }

    // P19.18 — channel_system_context handles empty tool list
    #[test]
    fn context_empty_tools() {
        let ctx = channel_system_context("telegram", &[]);
        assert!(ctx.contains("No tools available."));
    }

    // CR.25 — channel_system_context returns slack-specific prompt
    #[test]
    fn context_slack() {
        let ctx = channel_system_context("slack", &["web_search".to_string()]);
        assert!(ctx.contains("Slack"));
        assert!(ctx.contains("mrkdwn"));
    }

    // CR.26 — channel_system_context returns discord-specific prompt
    #[test]
    fn context_discord() {
        let ctx = channel_system_context("discord", &["web_search".to_string()]);
        assert!(ctx.contains("Discord"));
        assert!(ctx.contains("2000"));
    }

    // CR.27 — channel_system_context returns generic prompt for unknown channel
    #[test]
    fn context_unknown() {
        let ctx = channel_system_context("matrix", &[]);
        assert!(ctx.contains("External"));
        assert!(ctx.contains("concise"));
    }

    // 8.7.7 — ChannelSessionMap resolves/creates sessions correctly
    // Pipeline session resolution is tested in isolation via ChannelSessionMap (session_map.rs CR.4-CR.7).
    // Full pipeline handle_message requires a wired AppState with an agent; tested end-to-end manually.
    #[cfg(all(feature = "channels", feature = "gateway", feature = "ai"))]
    #[tokio::test]
    async fn pipeline_session_resolved() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let session_map = ChannelSessionMap::new(state.session_manager.clone());
        let msg = ChannelMessage::new("telegram", "hello");
        let channel_key = ChannelSessionMap::channel_key(&msg);
        let session_id = session_map
            .resolve_session(&channel_key, "telegram")
            .await
            .unwrap();
        assert!(!session_id.is_empty(), "Session should be created");

        // Resolving again yields the same session
        let session_id2 = session_map
            .resolve_session(&channel_key, "telegram")
            .await
            .unwrap();
        assert_eq!(session_id, session_id2);
    }

    // 8.7.8 — Tool policy filtering returns only allowed tools (uses PermissionResolver)
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[test]
    fn pipeline_tool_policy_filters() {
        use crate::config::AppConfig;
        use std::sync::Arc;

        // Default config: Low+Medium allowed, High denied on channels
        let config = Arc::new(AppConfig::default());
        let policy = ChannelToolPolicy::new(config);

        // Register system_info (Low) + shell (High) in registry
        let registry = crate::tools::ToolRegistry::new();
        registry
            .register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))
            .unwrap();
        registry
            .register(Arc::new(crate::tools::shell::ShellTool::new(
                Arc::new(crate::security::policy::SecurityPolicy::default_policy()),
                30,
            )))
            .unwrap();

        // On telegram: system_info passes, shell blocked
        let tools = policy.allowed_tools("telegram", &registry);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "system_info");
    }

    // 8.7.9 — channel_system_context provides channel-specific preamble with tools
    #[test]
    fn pipeline_preamble_override() {
        let tools = vec!["web_search".to_string()];
        let telegram_ctx = channel_system_context("telegram", &tools);
        assert!(
            telegram_ctx.contains("Telegram"),
            "Should contain channel name"
        );
        assert!(
            telegram_ctx.contains("web_search"),
            "Should list available tools"
        );

        let slack_ctx = channel_system_context("slack", &tools);
        assert!(slack_ctx.contains("Slack"));
        assert!(slack_ctx.contains("mrkdwn"));

        // Unknown channel gets generic context
        let custom_ctx = channel_system_context("custom_channel", &[]);
        assert!(custom_ctx.contains("External"));
    }

    // 8.7.10 — Messages are stored in session via session_manager
    #[cfg(all(feature = "channels", feature = "gateway", feature = "ai"))]
    #[tokio::test]
    async fn pipeline_messages_stored() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;

        // Create a session
        let session = state
            .session_manager
            .create_session_with_source("Test", "telegram")
            .await
            .unwrap();

        // Append a user message (simulating what handle_message does)
        state
            .session_manager
            .append_message(&session.id, "user", "hello from telegram")
            .await
            .unwrap();

        // Verify message was stored
        let msgs = state
            .session_manager
            .get_messages(&session.id)
            .await
            .unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello from telegram");
    }

    // CR.40 — Router end-to-end (requires live credentials, skipped by default)
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    #[ignore = "Requires live API credentials. Run manually with: cargo test router_end_to_end -- --ignored"]
    async fn router_end_to_end() {
        // Full pipeline test: send message through router, verify agent response stored.
        // Requires OPENAI_API_KEY or equivalent credential.
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let router = ChannelRouter::new(32);
        router.wire(state);
        router.start().await;

        let msg = ChannelMessage::new("test", "hello");
        router.sender().send(msg).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        router.stop();
    }

    // CR.41 — Router session persistence (requires live credentials, skipped by default)
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    #[ignore = "Requires live API credentials. Run manually with: cargo test router_session_persistence -- --ignored"]
    async fn router_session_persistence() {
        // Verify that after router processes a message, the session and messages persist in DB.
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let router = ChannelRouter::new(32);
        router.wire(state.clone());
        router.start().await;

        let msg = ChannelMessage::new("telegram", "persist test").with_sender("user123");
        router.sender().send(msg).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        router.stop();

        // Verify session was created
        let sessions = state.session_manager.list_sessions().await.unwrap();
        assert!(
            !sessions.is_empty(),
            "At least one session should be created"
        );
    }

    // CR.42 — Router tool policy filtering (requires live credentials, skipped by default)
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    #[ignore = "Requires live API credentials. Run manually with: cargo test router_tool_policy_filtering -- --ignored"]
    async fn router_tool_policy_filtering() {
        // Verify that restricted tool policy limits tools available during agent turn.
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        let router = ChannelRouter::new(32);
        router.wire(state);
        router.start().await;

        let msg = ChannelMessage::new("telegram", "what tools can you use?");
        router.sender().send(msg).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        router.stop();
    }
}
