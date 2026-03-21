use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::ai::adapter::{ToolCallEvent, ToolCallPhase};
use crate::ai::prompt::AssemblyRequest;
use crate::ai::resolve_agent;
use crate::gateway::state::AppState;

#[derive(Debug, Deserialize)]
struct WsRequest {
    prompt: String,
    session_id: Option<String>,
    model: Option<String>,
    #[serde(default)]
    delegation: Option<bool>,
}

/// Tagged enum for all outbound WebSocket messages.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum WsOutbound {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "tool_call")]
    ToolCall {
        call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        call_id: String,
        tool_name: String,
        output: String,
        success: bool,
        duration_ms: u64,
    },
    #[serde(rename = "notification")]
    Notification {
        event_type: String,
        job_id: String,
        job_name: String,
        message: Option<String>,
        status: Option<String>,
        error: Option<String>,
    },
    #[serde(rename = "channel_message")]
    ChannelMessage {
        channel: String,
        sender: String,
        session_id: String,
        content_preview: String,
        role: String,
    },
    #[serde(rename = "channel_connected")]
    ChannelConnected { channel: String },
    #[serde(rename = "channel_disconnected")]
    ChannelDisconnected { channel: String, reason: String },
    #[serde(rename = "channel_reconnecting")]
    ChannelReconnecting { channel: String, attempt: u32 },
    #[serde(rename = "delegation_started")]
    DelegationStarted {
        delegation_id: String,
        agent_count: usize,
        agents: Vec<DelegationAgentWs>,
    },
    #[serde(rename = "agent_progress")]
    AgentProgress {
        delegation_id: String,
        agent_id: String,
        tool_uses: u32,
        tokens_used: u64,
        current_activity: String,
    },
    #[serde(rename = "agent_completed")]
    AgentCompleted {
        delegation_id: String,
        agent_id: String,
        status: String,
        duration_ms: u64,
        tool_uses: u32,
        tokens_used: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "delegation_completed")]
    DelegationDone {
        delegation_id: String,
        total_duration_ms: u64,
        total_tokens: u64,
    },
    #[serde(rename = "workflow_started")]
    WorkflowStarted { workflow_id: String, run_id: String },
    #[serde(rename = "workflow_step_completed")]
    WorkflowStepCompleted {
        workflow_id: String,
        run_id: String,
        step_name: String,
        success: bool,
    },
    #[serde(rename = "workflow_completed")]
    WorkflowCompleted {
        workflow_id: String,
        run_id: String,
        status: String,
    },
    #[serde(rename = "channel_agent_started")]
    ChannelAgentStarted {
        channel: String,
        session_id: String,
        sender: String,
    },
    #[serde(rename = "channel_agent_completed")]
    ChannelAgentCompleted { channel: String, session_id: String },
    #[serde(rename = "approval_request")]
    ApprovalRequest {
        approval_id: String,
        call_id: String,
        tool_name: String,
        args_summary: String,
        risk_level: String,
        reason: String,
        timeout_secs: u64,
    },
    #[serde(rename = "approval_resolved")]
    ApprovalResolved {
        approval_id: String,
        decision: String,
        auto: bool,
    },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "warning")]
    Warning { warning: String },
    #[serde(rename = "error")]
    Error { error: String },
}

/// Agent info for WS delegation messages.
#[derive(Debug, Serialize)]
pub(crate) struct DelegationAgentWs {
    pub id: String,
    pub description: String,
}

#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/ws/notifications", tag = "WebSocket",
    responses(
        (status = 101, description = "WebSocket upgrade for real-time notifications (scheduler events, channel messages)")
    )
))]
pub async fn ws_notifications(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_notifications(socket, state))
}

async fn handle_notifications(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.event_bus.subscribe();

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(crate::event_bus::AppEvent::SchedulerNotification { job_id, job_name, message }) => {
                        let outbound = WsOutbound::Notification {
                            event_type: "scheduler_notification".into(),
                            job_id,
                            job_name,
                            message: Some(message),
                            status: None,
                            error: None,
                        };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::SchedulerJobCompleted { job_id, job_name, status, error }) => {
                        let outbound = WsOutbound::Notification {
                            event_type: "scheduler_job_completed".into(),
                            job_id,
                            job_name,
                            message: None,
                            status: Some(status),
                            error,
                        };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::ChannelMessageReceived { channel, sender, session_id, content_preview, role }) => {
                        let outbound = WsOutbound::ChannelMessage {
                            channel,
                            sender,
                            session_id,
                            content_preview,
                            role,
                        };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::ChannelConnected { channel }) => {
                        let outbound = WsOutbound::ChannelConnected { channel };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::ChannelDisconnected { channel, reason }) => {
                        let outbound = WsOutbound::ChannelDisconnected { channel, reason };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::ChannelReconnecting { channel, attempt }) => {
                        let outbound = WsOutbound::ChannelReconnecting { channel, attempt };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::ChannelAgentStarted { channel, session_id, sender }) => {
                        let outbound = WsOutbound::ChannelAgentStarted { channel, session_id, sender };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::ChannelAgentCompleted { channel, session_id }) => {
                        let outbound = WsOutbound::ChannelAgentCompleted { channel, session_id };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::ApprovalRequested { approval_id, call_id, tool_name, args_summary, risk_level, reason, timeout_secs }) => {
                        let outbound = WsOutbound::ApprovalRequest { approval_id, call_id, tool_name, args_summary, risk_level, reason, timeout_secs };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::HeartbeatAlert { message }) => {
                        let outbound = WsOutbound::Notification {
                            event_type: "heartbeat_alert".into(),
                            job_id: String::new(),
                            job_name: "heartbeat".into(),
                            message: Some(message),
                            status: None,
                            error: None,
                        };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::WorkflowStarted { workflow_id, run_id }) => {
                        let outbound = WsOutbound::WorkflowStarted { workflow_id, run_id };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::WorkflowStepCompleted { workflow_id, run_id, step_name, success }) => {
                        let outbound = WsOutbound::WorkflowStepCompleted { workflow_id, run_id, step_name, success };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::WorkflowCompleted { workflow_id, run_id, status }) => {
                        let outbound = WsOutbound::WorkflowCompleted { workflow_id, run_id, status };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::Shutdown) => {
                        break;
                    }
                    Ok(_) => {
                        // Ignore other events on this endpoint
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("notification WS lagged by {n} messages");
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/ws/chat", tag = "WebSocket",
    responses(
        (status = 101, description = "WebSocket upgrade for interactive chat with tool call streaming")
    )
))]
pub async fn ws_chat(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn send_outbound(socket: &mut WebSocket, msg: &WsOutbound) {
    if let Ok(json) = serde_json::to_string(msg) {
        let _ = socket.send(Message::Text(json.into())).await;
    }
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let request: WsRequest = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: format!("invalid JSON: {e}"),
                    },
                )
                .await;
                continue;
            }
        };

        // Build context parts via ContextBuilder
        let (history, _memories, _user_obs) = match state
            .context_builder
            .build_parts(request.session_id.as_deref(), &request.prompt)
            .await
        {
            Ok(ctx) => ctx,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: format!("context build failed: {e}"),
                    },
                )
                .await;
                continue;
            }
        };

        // Get conversation summary for resumed sessions
        let summary = if let Some(ref sid) = request.session_id {
            state
                .session_manager
                .get_context_info(sid)
                .await
                .ok()
                .and_then(|(_, _, s)| s)
        } else {
            None
        };

        // Assemble preamble via PromptStrategy
        let config = state.config.load_full();
        let model_display = request.model.as_deref().unwrap_or("default");
        let assembly_request = AssemblyRequest {
            boot_context: state.boot_context.clone(),
            model_display: model_display.into(),
            session_id: request.session_id.clone(),
            user_message: Some(request.prompt.clone()),
            conversation_summary: summary,
            channel_hint: None,
            tool_count: state.tools.len(),
            skill_count: state.skill_registry.list().await.len(),
            version: config.identity_name.clone(),
        };
        let merged_preamble = match state.prompt_strategy.assemble(&assembly_request).await {
            Ok(p) => p,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: format!("prompt assembly failed: {e}"),
                    },
                )
                .await;
                continue;
            }
        };
        debug!(
            "WS chat: session={}, history={} msgs, preamble={}B, prompt='{}'",
            request.session_id.as_deref().unwrap_or("none"),
            history.len(),
            merged_preamble.len(),
            &request.prompt[..request.prompt.len().min(80)]
        );

        // Delegation path: decompose into sub-agents with progress tracking
        if request.delegation == Some(true) {
            handle_delegation(
                &mut socket,
                &state,
                &request.prompt,
                request.session_id.as_deref(),
            )
            .await;
            continue;
        }

        // Create per-request broadcast channel for tool events
        let (tool_tx, mut tool_rx) = broadcast::channel::<ToolCallEvent>(128);

        let agent = match resolve_agent(
            request.model.as_deref(),
            &state,
            Some(tool_tx),
            Some(&merged_preamble),
            "desktop",
        )
        .await
        {
            Ok(a) => a,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: e.to_string(),
                    },
                )
                .await;
                continue;
            }
        };

        // Note: user message is stored by the frontend via POST /sessions/{id}/messages
        // before the WS stream starts. Do not duplicate here.

        // Spawn agent work in background with reasoning engine
        let prompt = request.prompt.clone();
        let reasoning_engine = state.reasoning_engine.clone();
        let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();
        let chat_start = std::time::Instant::now();
        let agent_timeout_secs = state.config.load().agent_timeout_secs;
        let agent_handle: JoinHandle<()> = tokio::spawn(async move {
            let timeout_result = tokio::time::timeout(
                std::time::Duration::from_secs(agent_timeout_secs),
                reasoning_engine.chat(&agent, &prompt, history),
            )
            .await;
            let result = match timeout_result {
                Ok(r) => r,
                Err(_) => Err(crate::ZeniiError::Agent(
                    "agent execution timed out".to_string(),
                )),
            };
            let _ = result_tx.send(result);
        });

        // Collect tool events for DB persistence
        let mut tool_events = Vec::new();

        // Concurrently forward tool events, wait for agent result, and detect client disconnect
        loop {
            tokio::select! {
                // H1: Detect client disconnect or approval responses during agent execution
                ws_msg = socket.recv() => {
                    match ws_msg {
                        Some(Ok(Message::Close(_))) | None => {
                            info!("WS: client disconnected during agent execution, aborting agent task");
                            agent_handle.abort();
                            break;
                        }
                        Some(Ok(Message::Text(text))) => {
                            // Handle approval_response messages from the client
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
                                && val.get("type").and_then(|v| v.as_str()) == Some("approval_response")
                            {
                                let approval_id = val.get("approval_id").and_then(|v| v.as_str()).unwrap_or("");
                                let decision_str = val.get("decision").and_then(|v| v.as_str()).unwrap_or("deny");
                                let decision = crate::security::approval::ApprovalDecision::from_str_lossy(decision_str);
                                if let Some(ref broker) = state.approval_broker {
                                    broker.resolve(approval_id, decision);
                                }
                            }
                        }
                        _ => {} // Ignore other messages during execution
                    }
                }
                event = tool_rx.recv() => {
                    match event {
                        Ok(evt) => {
                            let outbound = match &evt.phase {
                                ToolCallPhase::Started { args } => WsOutbound::ToolCall {
                                    call_id: evt.call_id.clone(),
                                    tool_name: evt.tool_name.clone(),
                                    args: args.clone(),
                                },
                                ToolCallPhase::Completed { output, success, duration_ms } => WsOutbound::ToolResult {
                                    call_id: evt.call_id.clone(),
                                    tool_name: evt.tool_name.clone(),
                                    output: output.clone(),
                                    success: *success,
                                    duration_ms: *duration_ms,
                                },
                                ToolCallPhase::Cached { output, success } => WsOutbound::ToolResult {
                                    call_id: evt.call_id.clone(),
                                    tool_name: evt.tool_name.clone(),
                                    output: output.clone(),
                                    success: *success,
                                    duration_ms: 0,
                                },
                                ToolCallPhase::ApprovalRequested { approval_id, reason, risk_level, timeout_secs } => {
                                    let args_summary = evt.tool_name.clone();
                                    WsOutbound::ApprovalRequest {
                                        approval_id: approval_id.clone(),
                                        call_id: evt.call_id.clone(),
                                        tool_name: evt.tool_name.clone(),
                                        args_summary,
                                        risk_level: risk_level.clone(),
                                        reason: reason.clone(),
                                        timeout_secs: *timeout_secs,
                                    }
                                }
                                ToolCallPhase::ApprovalResolved { approval_id, decision } => {
                                    WsOutbound::ApprovalResolved {
                                        approval_id: approval_id.clone(),
                                        decision: decision.clone(),
                                        auto: false,
                                    }
                                }
                            };
                            send_outbound(&mut socket, &outbound).await;
                            tool_events.push(evt);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // All senders dropped — agent is done, wait for result
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("tool event receiver lagged by {n} messages");
                            send_outbound(&mut socket, &WsOutbound::Warning {
                                warning: format!("{n} tool events were dropped due to high volume"),
                            }).await;
                        }
                    }
                }
                result = &mut result_rx => {
                    // Drain any remaining tool events that arrived before/during result
                    while let Ok(evt) = tool_rx.try_recv() {
                        let outbound = match &evt.phase {
                            ToolCallPhase::Started { args } => WsOutbound::ToolCall {
                                call_id: evt.call_id.clone(),
                                tool_name: evt.tool_name.clone(),
                                args: args.clone(),
                            },
                            ToolCallPhase::Completed { output, success, duration_ms } => WsOutbound::ToolResult {
                                call_id: evt.call_id.clone(),
                                tool_name: evt.tool_name.clone(),
                                output: output.clone(),
                                success: *success,
                                duration_ms: *duration_ms,
                            },
                            ToolCallPhase::Cached { output, success } => WsOutbound::ToolResult {
                                call_id: evt.call_id.clone(),
                                tool_name: evt.tool_name.clone(),
                                output: output.clone(),
                                success: *success,
                                duration_ms: 0,
                            },
                            ToolCallPhase::ApprovalRequested { approval_id, reason, risk_level, timeout_secs } => {
                                WsOutbound::ApprovalRequest {
                                    approval_id: approval_id.clone(),
                                    call_id: evt.call_id.clone(),
                                    tool_name: evt.tool_name.clone(),
                                    args_summary: evt.tool_name.clone(),
                                    risk_level: risk_level.clone(),
                                    reason: reason.clone(),
                                    timeout_secs: *timeout_secs,
                                }
                            }
                            ToolCallPhase::ApprovalResolved { approval_id, decision } => {
                                WsOutbound::ApprovalResolved {
                                    approval_id: approval_id.clone(),
                                    decision: decision.clone(),
                                    auto: false,
                                }
                            }
                        };
                        send_outbound(&mut socket, &outbound).await;
                        tool_events.push(evt);
                    }

                    match result {
                        Ok(Ok(chat_result)) => {
                            let duration_ms = chat_start.elapsed().as_millis() as u64;
                            let response = chat_result.response;
                            send_outbound(&mut socket, &WsOutbound::Text { content: response.clone() }).await;

                            // Log usage
                            let record = crate::logging::UsageRecord {
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                session_id: request.session_id.clone(),
                                model_id: model_display.to_string(),
                                provider_id: model_display.split(':').next().unwrap_or("unknown").to_string(),
                                input_tokens: chat_result.usage.input_tokens,
                                output_tokens: chat_result.usage.output_tokens,
                                total_tokens: chat_result.usage.total_tokens,
                                cached_input_tokens: chat_result.usage.cached_input_tokens,
                                tool_calls_count: tool_events.len() as u32,
                                duration_ms,
                                context_level: "Full".into(),
                                binary: state.usage_logger.binary_name().to_string(),
                                success: true,
                            };
                            let logger = state.usage_logger.clone();
                            tokio::spawn(async move {
                                let _ = logger.log(&record).await;
                            });

                            // Store assistant response and tool calls (retry once on failure)
                            if let Some(ref sid) = request.session_id {
                                info!(
                                    "WS: storing assistant response for session={sid}, len={}",
                                    response.len()
                                );
                                let mut msg = state
                                    .session_manager
                                    .append_message(sid, "assistant", &response)
                                    .await;

                                // Retry once after 100ms on failure
                                if msg.is_err() {
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                    msg = state
                                        .session_manager
                                        .append_message(sid, "assistant", &response)
                                        .await;
                                }

                                match &msg {
                                    Ok(m) => info!(
                                        "WS: assistant message stored OK: id={}, session={}",
                                        m.id, m.session_id
                                    ),
                                    Err(e) => {
                                        warn!(
                                            "WS: FAILED to store assistant message for session={sid}: {e}"
                                        );
                                        send_outbound(&mut socket, &WsOutbound::Warning {
                                            warning: "message could not be saved to history".into(),
                                        }).await;
                                    }
                                }

                                if let Ok(msg) = msg && !tool_events.is_empty() {
                                    let _ = state
                                        .session_manager
                                        .store_tool_calls(&msg.id, sid, &tool_events)
                                        .await;
                                }

                                // Auto-extract facts from the conversation
                                let _ = state
                                    .context_builder
                                    .extract_facts(&request.prompt, &response, Some(sid))
                                    .await;
                            }

                            send_outbound(&mut socket, &WsOutbound::Done).await;
                        }
                        Ok(Err(e)) => {
                            send_outbound(&mut socket, &WsOutbound::Error { error: e.to_string() }).await;
                        }
                        Err(_) => {
                            send_outbound(&mut socket, &WsOutbound::Error { error: "agent task cancelled".into() }).await;
                        }
                    }
                    break;
                }
            }
        }
    }
}

/// Handle a delegation request: decompose into sub-agents, stream progress events.
async fn handle_delegation(
    socket: &mut WebSocket,
    state: &Arc<AppState>,
    prompt: &str,
    session_id: Option<&str>,
) {
    let mut event_rx = state.event_bus.subscribe();

    // Spawn the delegation in background
    let state_clone = state.clone();
    let prompt_owned = prompt.to_string();
    let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();

    // A.2: Store handle so we can abort on disconnect
    let delegation_handle = tokio::spawn(async move {
        let result = state_clone
            .coordinator
            .delegate(&prompt_owned, &state_clone, "desktop")
            .await;
        let _ = result_tx.send(result);
    });

    // A.1: Track our delegation_id to filter events
    let mut my_delegation_id: Option<String> = None;

    // Forward delegation events until completion
    loop {
        tokio::select! {
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) | None => {
                        // A.2: Cancel delegation on client disconnect
                        info!("WS: client disconnected during delegation, aborting");
                        delegation_handle.abort();
                        break;
                    }
                    _ => {}
                }
            }
            event = event_rx.recv() => {
                match event {
                    Ok(crate::event_bus::AppEvent::DelegationStarted { delegation_id, agents }) => {
                        // A.1: Claim this delegation_id
                        my_delegation_id = Some(delegation_id.clone());
                        let ws_agents: Vec<DelegationAgentWs> = agents
                            .iter()
                            .map(|a| DelegationAgentWs {
                                id: a.id.clone(),
                                description: a.description.clone(),
                            })
                            .collect();
                        let count = ws_agents.len();
                        send_outbound(socket, &WsOutbound::DelegationStarted {
                            delegation_id,
                            agent_count: count,
                            agents: ws_agents,
                        }).await;
                    }
                    // A.1: Filter all delegation events by our delegation_id
                    Ok(crate::event_bus::AppEvent::SubAgentProgress { delegation_id, agent_id, tool_uses, tokens_used, current_activity })
                        if my_delegation_id.as_deref() == Some(delegation_id.as_str()) =>
                    {
                        send_outbound(socket, &WsOutbound::AgentProgress {
                            delegation_id,
                            agent_id,
                            tool_uses,
                            tokens_used,
                            current_activity,
                        }).await;
                    }
                    Ok(crate::event_bus::AppEvent::SubAgentCompleted { delegation_id, agent_id, status, duration_ms, tool_uses, tokens_used })
                        if my_delegation_id.as_deref() == Some(delegation_id.as_str()) =>
                    {
                        send_outbound(socket, &WsOutbound::AgentCompleted {
                            delegation_id,
                            agent_id,
                            status,
                            duration_ms,
                            tool_uses,
                            tokens_used,
                            error: None,
                        }).await;
                    }
                    // A.3: Forward error detail and real duration_ms from SubAgentFailed
                    Ok(crate::event_bus::AppEvent::SubAgentFailed { delegation_id, agent_id, error, tool_uses, duration_ms })
                        if my_delegation_id.as_deref() == Some(delegation_id.as_str()) =>
                    {
                        send_outbound(socket, &WsOutbound::AgentCompleted {
                            delegation_id,
                            agent_id,
                            status: "failed".into(),
                            duration_ms,
                            tool_uses,
                            tokens_used: 0,
                            error: Some(error.clone()),
                        }).await;
                        debug!("Sub-agent failed: {error}");
                    }
                    Ok(crate::event_bus::AppEvent::DelegationCompleted { delegation_id, total_duration_ms, total_tokens })
                        if my_delegation_id.as_deref() == Some(delegation_id.as_str()) =>
                    {
                        send_outbound(socket, &WsOutbound::DelegationDone {
                            delegation_id,
                            total_duration_ms,
                            total_tokens,
                        }).await;
                    }
                    Ok(_) => {} // Ignore other events / non-matching delegation_ids
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("delegation WS lagged by {n} messages");
                    }
                }
            }
            result = &mut result_rx => {
                match result {
                    Ok(Ok(delegation_result)) => {
                        let response = delegation_result.aggregated_response.clone();
                        send_outbound(socket, &WsOutbound::Text {
                            content: response.clone(),
                        })
                        .await;

                        // Persist assistant response to session history
                        if let Some(sid) = session_id {
                            let mut msg = state
                                .session_manager
                                .append_message(sid, "assistant", &response)
                                .await;
                            if msg.is_err() {
                                tokio::time::sleep(std::time::Duration::from_millis(100))
                                    .await;
                                msg = state
                                    .session_manager
                                    .append_message(sid, "assistant", &response)
                                    .await;
                            }
                            if let Err(e) = &msg {
                                warn!(
                                    "WS: FAILED to store delegation response for session={sid}: {e}"
                                );
                                send_outbound(socket, &WsOutbound::Warning {
                                    warning: "message could not be saved to history".into(),
                                })
                                .await;
                            }

                            // Store delegation details linked to the assistant message
                            if let Ok(ref msg) = msg
                                && let Err(e) = state
                                    .session_manager
                                    .store_delegation(&msg.id, sid, &delegation_result)
                                    .await
                            {
                                warn!("WS: FAILED to store delegation details for session={sid}: {e}");
                            }
                        }

                        send_outbound(socket, &WsOutbound::Done).await;
                    }
                    Ok(Err(e)) => {
                        send_outbound(socket, &WsOutbound::Error {
                            error: e.to_string(),
                        }).await;
                    }
                    Err(_) => {
                        send_outbound(socket, &WsOutbound::Error {
                            error: "delegation task cancelled".into(),
                        }).await;
                    }
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures::{SinkExt, StreamExt};
    use serde_json::json;
    use tokio_tungstenite::tungstenite;

    use super::*;
    use crate::gateway::routes::build_router;
    use crate::gateway::state::AppState;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    /// Spawn an axum server on a random port and return the port number.
    async fn spawn_server(state: Arc<AppState>) -> u16 {
        let router = build_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        port
    }

    // 8.6.1.15 — WsOutbound::Notification serializes correctly
    #[test]
    fn ws_outbound_notification_serializes() {
        let msg = WsOutbound::Notification {
            event_type: "scheduler_notification".into(),
            job_id: "j1".into(),
            job_name: "test".into(),
            message: Some("hello".into()),
            status: None,
            error: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "notification");
        assert_eq!(json["event_type"], "scheduler_notification");
        assert_eq!(json["job_id"], "j1");
        assert_eq!(json["message"], "hello");
    }

    // 8.6.1.16 — WS notifications endpoint upgrade succeeds
    #[tokio::test]
    async fn ws_notifications_upgrade_succeeds() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/notifications");
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(result.is_ok(), "Notification WS upgrade should succeed");
    }

    // 8.6.1.17 — WS notifications forwards scheduler events
    #[tokio::test]
    async fn ws_notifications_forwards_events() {
        let (_dir, state) = test_state().await;
        let bus = state.event_bus.clone();
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/notifications");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // Give the WS handler time to subscribe
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Publish a scheduler notification event
        bus.publish(crate::event_bus::AppEvent::SchedulerNotification {
            job_id: "j1".into(),
            job_name: "test_job".into(),
            message: "hello from scheduler".into(),
        })
        .unwrap();

        // Read the forwarded message
        let resp = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await;

        assert!(resp.is_ok(), "Should receive notification within timeout");
        let msg = resp.unwrap().unwrap().unwrap();
        let text = msg.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "notification");
        assert_eq!(parsed["event_type"], "scheduler_notification");
        assert_eq!(parsed["job_id"], "j1");
        assert_eq!(parsed["message"], "hello from scheduler");
    }

    // IN.10 — WsOutbound::ChannelMessage serializes correctly
    #[test]
    fn ws_outbound_channel_message_serializes() {
        let msg = WsOutbound::ChannelMessage {
            channel: "telegram".into(),
            sender: "user123".into(),
            session_id: "sess-abc".into(),
            content_preview: "Hello there".into(),
            role: "user".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "channel_message");
        assert_eq!(json["channel"], "telegram");
        assert_eq!(json["sender"], "user123");
        assert_eq!(json["session_id"], "sess-abc");
        assert_eq!(json["content_preview"], "Hello there");
        assert_eq!(json["role"], "user");
    }

    // SUP.5 — WsOutbound::ChannelConnected serializes correctly
    #[test]
    fn ws_outbound_channel_connected_serializes() {
        let msg = WsOutbound::ChannelConnected {
            channel: "telegram".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "channel_connected");
        assert_eq!(json["channel"], "telegram");
    }

    // SUP.6 — WsOutbound::ChannelDisconnected serializes correctly
    #[test]
    fn ws_outbound_channel_disconnected_serializes() {
        let msg = WsOutbound::ChannelDisconnected {
            channel: "slack".into(),
            reason: "network error".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "channel_disconnected");
        assert_eq!(json["channel"], "slack");
        assert_eq!(json["reason"], "network error");
    }

    // SUP.7 — WsOutbound::ChannelReconnecting serializes correctly
    #[test]
    fn ws_outbound_channel_reconnecting_serializes() {
        let msg = WsOutbound::ChannelReconnecting {
            channel: "discord".into(),
            attempt: 3,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "channel_reconnecting");
        assert_eq!(json["channel"], "discord");
        assert_eq!(json["attempt"], 3);
    }

    // TV.11 — WsOutbound::Text serializes to {"type":"text","content":"..."}
    #[test]
    fn ws_outbound_text_serializes() {
        let msg = WsOutbound::Text {
            content: "hello".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["content"], "hello");
    }

    // TV.12 — WsOutbound::ToolCall serializes with call_id, tool_name, args
    #[test]
    fn ws_outbound_tool_call_serializes() {
        let msg = WsOutbound::ToolCall {
            call_id: "abc".into(),
            tool_name: "WebSearch".into(),
            args: json!({"query": "rust"}),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "tool_call");
        assert_eq!(json["call_id"], "abc");
        assert_eq!(json["tool_name"], "WebSearch");
        assert_eq!(json["args"]["query"], "rust");
    }

    // TV.13 — WsOutbound::ToolResult serializes with all fields
    #[test]
    fn ws_outbound_tool_result_serializes() {
        let msg = WsOutbound::ToolResult {
            call_id: "abc".into(),
            tool_name: "WebSearch".into(),
            output: "results".into(),
            success: true,
            duration_ms: 150,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["call_id"], "abc");
        assert_eq!(json["tool_name"], "WebSearch");
        assert_eq!(json["output"], "results");
        assert_eq!(json["success"], true);
        assert_eq!(json["duration_ms"], 150);
    }

    // TV.14 — WsOutbound::Done serializes to {"type":"done"}
    #[test]
    fn ws_outbound_done_serializes() {
        let msg = WsOutbound::Done;
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "done");
    }

    // TC-I1 — Cached phase maps to ToolResult with duration_ms=0
    #[test]
    fn tc_i1_cached_phase_maps_to_tool_result() {
        // When a Cached event arrives, the WS handler maps it to ToolResult with duration_ms: 0
        let cached_event = ToolCallEvent {
            call_id: "c1".into(),
            tool_name: "web_search".into(),
            phase: ToolCallPhase::Cached {
                output: "cached result".into(),
                success: true,
            },
        };
        let outbound = match &cached_event.phase {
            ToolCallPhase::Cached { output, success } => WsOutbound::ToolResult {
                call_id: cached_event.call_id.clone(),
                tool_name: cached_event.tool_name.clone(),
                output: output.clone(),
                success: *success,
                duration_ms: 0,
            },
            _ => unreachable!(),
        };
        let json = serde_json::to_value(&outbound).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["call_id"], "c1");
        assert_eq!(json["tool_name"], "web_search");
        assert_eq!(json["output"], "cached result");
        assert_eq!(json["success"], true);
        assert_eq!(json["duration_ms"], 0);
    }

    // TV.15 — WsOutbound::Error serializes with error field
    #[test]
    fn ws_outbound_error_serializes() {
        let msg = WsOutbound::Error {
            error: "oops".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["error"], "oops");
    }

    // AUDIT-C4 — WsOutbound::Warning serializes with warning field
    #[test]
    fn ws_outbound_warning_serializes() {
        let msg = WsOutbound::Warning {
            warning: "message could not be saved".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "warning");
        assert_eq!(json["warning"], "message could not be saved");
    }

    // AUDIT-H2 — WsOutbound::Warning for lagged events serializes correctly
    #[test]
    fn ws_outbound_warning_lagged_serializes() {
        let msg = WsOutbound::Warning {
            warning: "5 tool events were dropped due to high volume".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "warning");
        assert!(json["warning"].as_str().unwrap().contains("dropped"));
    }

    // WF.1 — WsOutbound::WorkflowStarted serializes correctly
    #[test]
    fn ws_outbound_workflow_started_serializes() {
        let msg = WsOutbound::WorkflowStarted {
            workflow_id: "wf1".into(),
            run_id: "run1".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "workflow_started");
        assert_eq!(json["workflow_id"], "wf1");
        assert_eq!(json["run_id"], "run1");
    }

    // WF.2 — WsOutbound::WorkflowStepCompleted serializes correctly
    #[test]
    fn ws_outbound_workflow_step_completed_serializes() {
        let msg = WsOutbound::WorkflowStepCompleted {
            workflow_id: "wf1".into(),
            run_id: "run1".into(),
            step_name: "fetch_data".into(),
            success: true,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "workflow_step_completed");
        assert_eq!(json["workflow_id"], "wf1");
        assert_eq!(json["run_id"], "run1");
        assert_eq!(json["step_name"], "fetch_data");
        assert_eq!(json["success"], true);
    }

    // WF.3 — WsOutbound::WorkflowCompleted serializes correctly
    #[test]
    fn ws_outbound_workflow_completed_serializes() {
        let msg = WsOutbound::WorkflowCompleted {
            workflow_id: "wf1".into(),
            run_id: "run1".into(),
            status: "completed".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "workflow_completed");
        assert_eq!(json["workflow_id"], "wf1");
        assert_eq!(json["run_id"], "run1");
        assert_eq!(json["status"], "completed");
    }

    // WF.4 — WS notifications forwards workflow events
    #[tokio::test]
    async fn ws_notifications_forwards_workflow_events() {
        let (_dir, state) = test_state().await;
        let bus = state.event_bus.clone();
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/notifications");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // Give the WS handler time to subscribe
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Publish workflow started event
        bus.publish(crate::event_bus::AppEvent::WorkflowStarted {
            workflow_id: "wf1".into(),
            run_id: "run1".into(),
        })
        .unwrap();

        let resp = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await;
        assert!(
            resp.is_ok(),
            "Should receive workflow_started within timeout"
        );
        let msg = resp.unwrap().unwrap().unwrap();
        let text = msg.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "workflow_started");
        assert_eq!(parsed["workflow_id"], "wf1");
        assert_eq!(parsed["run_id"], "run1");
    }

    // TV.16 — WS upgrade still succeeds
    #[tokio::test]
    async fn ws_upgrade_succeeds() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/chat");
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(result.is_ok(), "WebSocket upgrade should succeed");
    }

    // TV.17 — WS invalid JSON still returns error
    #[tokio::test]
    async fn ws_invalid_json_returns_error() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/chat");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        ws.send(tungstenite::Message::Text("not json".into()))
            .await
            .unwrap();

        let resp = ws.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
        assert!(parsed["error"].as_str().unwrap().contains("invalid JSON"));
    }

    // TA.4 — WsOutbound::ChannelAgentStarted serializes correctly
    #[test]
    fn ws_outbound_channel_agent_started_serializes() {
        let msg = WsOutbound::ChannelAgentStarted {
            channel: "telegram".into(),
            session_id: "sess-1".into(),
            sender: "user42".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "channel_agent_started");
        assert_eq!(json["channel"], "telegram");
        assert_eq!(json["session_id"], "sess-1");
        assert_eq!(json["sender"], "user42");
    }

    // TA.5 — WsOutbound::ChannelAgentCompleted serializes correctly
    #[test]
    fn ws_outbound_channel_agent_completed_serializes() {
        let msg = WsOutbound::ChannelAgentCompleted {
            channel: "slack".into(),
            session_id: "sess-2".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "channel_agent_completed");
        assert_eq!(json["channel"], "slack");
        assert_eq!(json["session_id"], "sess-2");
    }

    // TA.6 — WsOutbound::ApprovalRequest serializes correctly
    #[test]
    fn ws_outbound_approval_request_serializes() {
        let msg = WsOutbound::ApprovalRequest {
            approval_id: "apr-1".into(),
            call_id: "call-1".into(),
            tool_name: "shell".into(),
            args_summary: "cargo build".into(),
            risk_level: "medium".into(),
            reason: "Command needs approval: cargo build".into(),
            timeout_secs: 120,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "approval_request");
        assert_eq!(json["approval_id"], "apr-1");
        assert_eq!(json["call_id"], "call-1");
        assert_eq!(json["tool_name"], "shell");
        assert_eq!(json["args_summary"], "cargo build");
        assert_eq!(json["risk_level"], "medium");
        assert_eq!(json["reason"], "Command needs approval: cargo build");
        assert_eq!(json["timeout_secs"], 120);
    }

    // TA.7 — WsOutbound::ApprovalResolved serializes correctly
    #[test]
    fn ws_outbound_approval_resolved_serializes() {
        let msg = WsOutbound::ApprovalResolved {
            approval_id: "apr-1".into(),
            decision: "approve".into(),
            auto: false,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "approval_resolved");
        assert_eq!(json["approval_id"], "apr-1");
        assert_eq!(json["decision"], "approve");
        assert_eq!(json["auto"], false);
    }

    // 4.2.3 — WS no API key returns credential error
    #[tokio::test]
    async fn ws_no_agent_returns_error() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/chat");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        let msg = serde_json::json!({"prompt": "hello"}).to_string();
        ws.send(tungstenite::Message::Text(msg.into()))
            .await
            .unwrap();

        let resp = ws.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
        // Default model is seeded (anthropic:claude-sonnet-4-6) but no API key exists,
        // so resolve_agent fails with a credential error.
        assert!(
            parsed["error"]
                .as_str()
                .unwrap()
                .contains("no API key found")
        );
    }
}
