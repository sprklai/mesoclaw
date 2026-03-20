#[allow(dead_code)]
mod app;
#[allow(dead_code)]
mod client;
#[allow(dead_code)]
mod event;
mod handler;
#[allow(dead_code)]
mod markdown;
#[allow(dead_code)]
mod theme;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use clap::Parser;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::{SinkExt, StreamExt};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio_tungstenite::tungstenite;
use tracing::{error, info, warn};

use crate::app::{
    App, ChatMessage, ChatStatus, ConnectionStatus, PluginListItem, SessionSummary, ToolEvent,
};
use crate::client::ZeniiClient;
use crate::event::{AppEvent, EventHandler, WsInbound};

#[derive(Parser)]
#[command(name = "zenii-tui", about = "Zenii Terminal UI")]
struct Args {
    #[arg(long, default_value = "127.0.0.1", env = "ZENII_HOST")]
    host: String,

    #[arg(long, default_value = "18981", env = "ZENII_PORT")]
    port: u16,

    #[arg(long, env = "ZENII_TOKEN")]
    token: Option<String>,
}

fn init_file_tracing() {
    let log_dir = directories::ProjectDirs::from("com", "sprklai", "zenii")
        .map(|d| d.data_dir().join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(&log_dir, "tui.log");
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("zenii_tui=debug,info"));
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true)
        .with_env_filter(env_filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set up tracing to OS-appropriate log dir (not terminal, since we own the terminal)
    init_file_tracing();

    // Set panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal(&mut io::stdout());
        original_hook(panic_info);
    }));

    let mut terminal = setup_terminal()?;
    let client = ZeniiClient::new(&args.host, args.port, args.token);

    let result = run_app(&mut terminal, &client).await;

    restore_terminal(&mut io::stdout())?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(stdout: &mut Stdout) -> Result<(), io::Error> {
    disable_raw_mode()?;
    crossterm::execute!(
        stdout,
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    client: &ZeniiClient,
) -> Result<(), String> {
    let mut app = App::new();

    // Health check
    match client.health().await {
        Ok(true) => {
            app.connection_status = ConnectionStatus::Connected;
            info!("Connected to gateway");
        }
        Ok(false) | Err(_) => {
            app.connection_status = ConnectionStatus::Disconnected;
            warn!("Gateway not available");
        }
    }

    // Load initial data
    load_sessions(&mut app, client).await;
    load_default_model(&mut app, client).await;

    // Check if onboarding is needed
    if app.connection_status == ConnectionStatus::Connected
        && let Ok(status) = client.get::<serde_json::Value>("/setup/status").await
        && status.get("needs_setup").and_then(|v| v.as_bool()) == Some(true)
    {
        // Pre-fill timezone
        if let Some(tz) = status.get("detected_timezone").and_then(|v| v.as_str()) {
            app.onboard_timezone.content = tz.to_string();
            app.onboard_timezone.cursor_pos = tz.len();
        }
        // Fetch providers
        if let Ok(providers) = client
            .get::<Vec<serde_json::Value>>("/providers/with-key-status")
            .await
        {
            app.onboard_providers = providers;
        }
        app.mode = app::AppMode::Onboard;
    }

    // Set up event handler with WS sender
    let (mut events, ws_tx) = EventHandler::new_with_ws_sender(Duration::from_millis(250));

    // Connect WebSocket for streaming
    spawn_ws_reader(client, ws_tx.clone());

    // Main loop
    loop {
        terminal
            .draw(|f| ui::render(f, &app))
            .map_err(|e| e.to_string())?;

        let Some(event) = events.next().await else {
            break;
        };

        match event {
            AppEvent::Key(key) => {
                let prev_notification = app.notification_text.take();
                handler::handle_key_event(&mut app, key);

                // Handle action signals from the handler
                if let Some(ref signal) = app.notification_text {
                    match signal.as_str() {
                        "__create_session__" => {
                            app.notification_text = None;
                            create_session(&mut app, client).await;
                        }
                        "__send_message__" => {
                            app.notification_text = None;
                            send_message(&mut app, client, &ws_tx).await;
                        }
                        "__onboard_save_key__" => {
                            app.notification_text = None;
                            let key_val = app.onboard_api_key.content.clone();
                            let provider_id = app.onboard_provider_id.clone();
                            let body = serde_json::json!({
                                "key": format!("api_key:{provider_id}"),
                                "value": key_val,
                            });
                            match client
                                .post::<_, serde_json::Value>("/credentials", &body)
                                .await
                            {
                                Ok(_) => {
                                    app.onboard_api_key.clear();
                                    // Refresh provider models
                                    if let Ok(providers) = client
                                        .get::<Vec<serde_json::Value>>("/providers/with-key-status")
                                        .await
                                    {
                                        if let Some(p) = providers
                                            .iter()
                                            .find(|p| p["id"].as_str() == Some(&provider_id))
                                        {
                                            app.onboard_models =
                                                p["models"].as_array().cloned().unwrap_or_default();
                                        }
                                        app.onboard_providers = providers;
                                    }
                                    app.onboard_selected_model = 0;
                                    app.onboard_step = app::OnboardStep::ModelSelect;
                                    app.onboard_error = None;
                                }
                                Err(e) => {
                                    app.onboard_error = Some(format!("Failed to save key: {e}"));
                                }
                            }
                        }
                        "__onboard_save_model__" => {
                            app.notification_text = None;
                            if let Some(model) = app.onboard_models.get(app.onboard_selected_model)
                            {
                                let model_id = model["model_id"].as_str().unwrap_or("").to_string();
                                let provider_id = app.onboard_provider_id.clone();
                                let body = serde_json::json!({
                                    "provider_id": provider_id,
                                    "model_id": model_id,
                                });
                                match client
                                    .put::<_, serde_json::Value>("/providers/default", &body)
                                    .await
                                {
                                    Ok(_) => {
                                        // Sync default model to config.toml
                                        let config_body = serde_json::json!({
                                            "provider_name": provider_id,
                                            "provider_type": provider_id,
                                            "provider_model_id": model_id,
                                        });
                                        let _ = client
                                            .put::<_, serde_json::Value>("/config", &config_body)
                                            .await;
                                        app.current_model = format!("{provider_id}:{model_id}");
                                        app.onboard_step = app::OnboardStep::Channels;
                                        app.onboard_error = None;
                                    }
                                    Err(e) => {
                                        app.onboard_error =
                                            Some(format!("Failed to set model: {e}"));
                                    }
                                }
                            }
                        }
                        "__onboard_save_channel_cred__" => {
                            app.notification_text = None;
                            let channels = app::ONBOARD_CHANNELS;
                            let channel = &channels[app.onboard_selected_channel];
                            let cred = &channel.credentials[app.onboard_channel_cred_idx];
                            let cred_key = format!("channel:{}:{}", channel.id, cred.0);
                            let value = app.onboard_channel_input.content.trim().to_string();
                            let body = serde_json::json!({
                                "key": cred_key,
                                "value": value,
                            });
                            match client
                                .post::<_, serde_json::Value>("/credentials", &body)
                                .await
                            {
                                Ok(_) => {
                                    app.onboard_channel_saved
                                        .insert(format!("channel:{}:{}", channel.id, cred.0));
                                    app.onboard_channel_input.clear();
                                    app.onboard_error = None;
                                    // Auto-advance to next credential field
                                    if app.onboard_channel_cred_idx < channel.credentials.len() - 1
                                    {
                                        app.onboard_channel_cred_idx += 1;
                                    }
                                }
                                Err(e) => {
                                    app.onboard_error = Some(format!("Failed to save: {e}"));
                                }
                            }
                        }
                        "__onboard_save_profile__" => {
                            app.notification_text = None;
                            app.onboard_saving = true;
                            let body = serde_json::json!({
                                "user_name": app.onboard_name.content.trim(),
                                "user_location": app.onboard_location.content.trim(),
                                "user_timezone": if app.onboard_timezone.content.trim().is_empty() {
                                    serde_json::Value::Null
                                } else {
                                    serde_json::Value::String(
                                        app.onboard_timezone.content.trim().to_string(),
                                    )
                                },
                            });
                            match client.put::<_, serde_json::Value>("/config", &body).await {
                                Ok(_) => {
                                    app.onboard_saving = false;
                                    app.mode = app::AppMode::SessionList;
                                    app.notification_text = Some("Setup complete!".into());
                                }
                                Err(e) => {
                                    app.onboard_saving = false;
                                    app.onboard_error = Some(format!("Failed to save: {e}"));
                                }
                            }
                        }
                        "__plugin_load__" => {
                            app.notification_text = None;
                            load_plugins(&mut app, client).await;
                        }
                        "__plugin_toggle__" => {
                            app.notification_text = None;
                            if let Some(idx) = app.selected_plugin
                                && let Some(plugin) = app.plugins.get(idx)
                            {
                                let name = plugin.name.clone();
                                match client
                                    .put::<_, serde_json::Value>(
                                        &format!("/plugins/{name}/toggle"),
                                        &serde_json::json!({}),
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        load_plugins(&mut app, client).await;
                                    }
                                    Err(e) => {
                                        app.plugin_error = Some(format!("Toggle failed: {e}"));
                                    }
                                }
                            }
                        }
                        "__plugin_remove__" => {
                            app.notification_text = None;
                            if let Some(idx) = app.selected_plugin
                                && let Some(plugin) = app.plugins.get(idx)
                            {
                                let name = plugin.name.clone();
                                match client.delete_req(&format!("/plugins/{name}")).await {
                                    Ok(()) => {
                                        load_plugins(&mut app, client).await;
                                        app.selected_plugin = None;
                                    }
                                    Err(e) => {
                                        app.plugin_error = Some(format!("Remove failed: {e}"));
                                    }
                                }
                            }
                        }
                        "__plugin_install_mode__" => {
                            app.notification_text =
                                Some("Enter plugin source URL or path, then press Enter:".into());
                        }
                        _ => {}
                    }
                }

                // Handle delete confirmation
                if app.confirm_delete {
                    // Waiting for 'y' confirmation — notification already set
                } else if prev_notification
                    .as_deref()
                    .is_some_and(|t| t == "Delete session? (y/n)")
                {
                    // 'y' was pressed (confirm_delete was just set to false after processing)
                    delete_session(&mut app, client).await;
                }

                // Handle refresh signal
                if app
                    .notification_text
                    .as_deref()
                    .is_some_and(|t| t == "Refreshing...")
                {
                    load_sessions(&mut app, client).await;
                    let sid = app.current_session_id.clone();
                    if let Some(sid) = sid {
                        load_messages(&mut app, client, &sid).await;
                    }
                    app.notification_text = Some("Refreshed".into());
                }

                // Load messages when entering chat mode
                if app.mode == app::AppMode::Chat
                    && app.current_session_id.is_none()
                    && let Some(idx) = app.selected_session
                    && let Some(session) = app.sessions.get(idx)
                {
                    let sid = session.id.clone();
                    load_messages(&mut app, client, &sid).await;
                    app.current_session_id = Some(sid);
                }
            }
            AppEvent::Resize(_, _) => {
                // Terminal handles redraw automatically
            }
            AppEvent::Tick => {
                // Clear transient notifications
                if app
                    .notification_text
                    .as_deref()
                    .is_some_and(|t| !t.starts_with("__") && t != "Delete session? (y/n)")
                {
                    // Keep for one more tick, then clear
                }
            }
            AppEvent::WsMessage(msg) => {
                handle_ws_message(&mut app, msg);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_ws_message(app: &mut App, msg: WsInbound) {
    match msg {
        WsInbound::Text { content } => {
            app.append_streaming_text(&content);
        }
        WsInbound::ToolCall {
            tool_name, args, ..
        } => {
            app.tool_events.push(ToolEvent {
                tool_name,
                args: args.to_string(),
                output: None,
                success: None,
                duration_ms: None,
            });
        }
        WsInbound::ToolResult {
            tool_name,
            output,
            success,
            duration_ms,
            ..
        } => {
            // Update the last matching tool event
            if let Some(evt) = app
                .tool_events
                .iter_mut()
                .rev()
                .find(|e| e.tool_name == tool_name && e.output.is_none())
            {
                evt.output = Some(output);
                evt.success = Some(success);
                evt.duration_ms = Some(duration_ms);
            }
        }
        WsInbound::DelegationStarted {
            delegation_id,
            agents,
            ..
        } => {
            app.start_delegation(
                delegation_id,
                agents.into_iter().map(|a| (a.id, a.description)).collect(),
            );
            app.chat_status = ChatStatus::Streaming;
        }
        WsInbound::AgentProgress {
            agent_id,
            tool_uses,
            tokens_used,
            current_activity,
            ..
        } => {
            app.update_agent(&agent_id, tool_uses, tokens_used, &current_activity);
        }
        WsInbound::AgentCompleted {
            agent_id,
            status,
            duration_ms,
            tool_uses,
            tokens_used,
            ..
        } => {
            app.complete_agent(&agent_id, &status, duration_ms, tool_uses, tokens_used);
        }
        WsInbound::DelegationCompleted { .. } => {
            // Keep tree visible until text+done arrives
        }
        WsInbound::Done => {
            app.delegation = None;
            app.flush_streaming_buffer();
        }
        WsInbound::Error { error } => {
            app.chat_status = ChatStatus::Error(error);
            app.streaming_buffer.clear();
        }
    }
}

async fn load_sessions(app: &mut App, client: &ZeniiClient) {
    match client.list_sessions().await {
        Ok(sessions) => {
            app.sessions = sessions
                .into_iter()
                .map(|v| SessionSummary {
                    id: v["id"].as_str().unwrap_or("").to_string(),
                    title: v["title"].as_str().unwrap_or("Untitled").to_string(),
                    updated_at: v["updated_at"].as_str().unwrap_or("").to_string(),
                    message_count: v["message_count"].as_i64().unwrap_or(0),
                })
                .collect();
            if !app.sessions.is_empty() && app.selected_session.is_none() {
                app.selected_session = Some(0);
            }
        }
        Err(e) => {
            warn!("Failed to load sessions: {e}");
        }
    }
}

async fn load_plugins(app: &mut App, client: &ZeniiClient) {
    app.plugin_loading = true;
    app.plugin_error = None;
    match client.get::<Vec<serde_json::Value>>("/plugins").await {
        Ok(list) => {
            app.plugins = list
                .into_iter()
                .map(|v| PluginListItem {
                    name: v["name"].as_str().unwrap_or("").to_string(),
                    version: v["version"].as_str().unwrap_or("0.0.0").to_string(),
                    description: v["description"].as_str().unwrap_or("").to_string(),
                    enabled: v["enabled"].as_bool().unwrap_or(false),
                    tools_count: v["tools_count"].as_u64().unwrap_or(0) as usize,
                    skills_count: v["skills_count"].as_u64().unwrap_or(0) as usize,
                })
                .collect();
            if !app.plugins.is_empty() && app.selected_plugin.is_none() {
                app.selected_plugin = Some(0);
            }
        }
        Err(e) => {
            app.plugin_error = Some(format!("Failed to load plugins: {e}"));
            warn!("Failed to load plugins: {e}");
        }
    }
    app.plugin_loading = false;
}

async fn load_default_model(app: &mut App, client: &ZeniiClient) {
    if let Ok(model) = client.get_default_model().await {
        app.current_model = model;
    }
}

async fn load_messages(app: &mut App, client: &ZeniiClient, session_id: &str) {
    match client.get_messages(session_id).await {
        Ok(messages) => {
            app.messages = messages
                .into_iter()
                .map(|v| ChatMessage {
                    role: v["role"].as_str().unwrap_or("user").to_string(),
                    content: v["content"].as_str().unwrap_or("").to_string(),
                    timestamp: v["created_at"].as_str().unwrap_or("").to_string(),
                    tool_calls: v["tool_calls"]
                        .as_array()
                        .map(|calls| {
                            calls
                                .iter()
                                .map(|c| ToolEvent {
                                    tool_name: c["tool_name"].as_str().unwrap_or("").to_string(),
                                    args: c["args"].to_string(),
                                    output: c["output"].as_str().map(|s| s.to_string()),
                                    success: c["success"].as_bool(),
                                    duration_ms: c["duration_ms"].as_u64(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect();
            app.scroll_offset = usize::MAX; // scroll to bottom
        }
        Err(e) => {
            warn!("Failed to load messages: {e}");
        }
    }
}

async fn create_session(app: &mut App, client: &ZeniiClient) {
    match client.create_session().await {
        Ok(session) => {
            let id = session["id"].as_str().unwrap_or("").to_string();
            let title = session["title"]
                .as_str()
                .unwrap_or("New Session")
                .to_string();
            app.sessions.insert(
                0,
                SessionSummary {
                    id: id.clone(),
                    title,
                    updated_at: String::new(),
                    message_count: 0,
                },
            );
            app.selected_session = Some(0);
            app.current_session_id = Some(id);
            app.messages.clear();
            app.enter_chat_mode();
            app.notification_text = Some("Session created".into());
        }
        Err(e) => {
            app.notification_text = Some(format!("Error: {e}"));
        }
    }
}

async fn delete_session(app: &mut App, client: &ZeniiClient) {
    let Some(idx) = app.selected_session else {
        return;
    };
    let Some(session) = app.sessions.get(idx) else {
        return;
    };
    let sid = session.id.clone();

    match client.delete_session(&sid).await {
        Ok(()) => {
            app.sessions.remove(idx);
            if app.current_session_id.as_deref() == Some(&sid) {
                app.current_session_id = None;
                app.messages.clear();
            }
            if app.sessions.is_empty() {
                app.selected_session = None;
            } else {
                app.selected_session = Some(idx.min(app.sessions.len() - 1));
            }
            app.notification_text = Some("Session deleted".into());
        }
        Err(e) => {
            app.notification_text = Some(format!("Delete failed: {e}"));
        }
    }
}

async fn send_message(
    app: &mut App,
    client: &ZeniiClient,
    ws_tx: &tokio::sync::mpsc::UnboundedSender<AppEvent>,
) {
    let content = app.input.take_content();
    if content.trim().is_empty() {
        app.chat_status = ChatStatus::Idle;
        return;
    }

    // Ensure we have a session
    if app.current_session_id.is_none() {
        match client.create_session().await {
            Ok(session) => {
                let id = session["id"].as_str().unwrap_or("").to_string();
                let title = session["title"]
                    .as_str()
                    .unwrap_or("New Session")
                    .to_string();
                app.sessions.insert(
                    0,
                    SessionSummary {
                        id: id.clone(),
                        title,
                        updated_at: String::new(),
                        message_count: 0,
                    },
                );
                app.selected_session = Some(0);
                app.current_session_id = Some(id);
            }
            Err(e) => {
                app.chat_status = ChatStatus::Error(format!("Failed to create session: {e}"));
                return;
            }
        }
    }

    let Some(session_id) = app.current_session_id.clone() else {
        app.chat_status = ChatStatus::Error("No session selected".into());
        return;
    };

    // Store user message via REST
    let _ = client.send_user_message(&session_id, &content).await;

    // Add to local display
    app.messages.push(ChatMessage {
        role: "user".to_string(),
        content: content.clone(),
        timestamp: String::new(),
        tool_calls: vec![],
    });
    app.scroll_offset = usize::MAX;

    // Send via WebSocket for streaming response
    let ws_url = client.ws_chat_url();
    let auth = client.auth_header_value();
    let prompt = content;
    let sid = session_id;
    let tx = ws_tx.clone();

    tokio::spawn(async move {
        let mut request =
            match tungstenite::client::IntoClientRequest::into_client_request(ws_url.as_str()) {
                Ok(r) => r,
                Err(e) => {
                    error!("Invalid WS URL: {e}");
                    let _ = tx.send(AppEvent::WsMessage(WsInbound::Error {
                        error: format!("Invalid WS URL: {e}"),
                    }));
                    return;
                }
            };

        if let Some(auth_val) = auth
            && let Ok(val) = auth_val.parse()
        {
            request.headers_mut().insert("authorization", val);
        }

        let ws = match tokio_tungstenite::connect_async(request).await {
            Ok((ws, _)) => ws,
            Err(e) => {
                let _ = tx.send(AppEvent::WsMessage(WsInbound::Error {
                    error: format!("WS connect failed: {e}"),
                }));
                return;
            }
        };

        let (mut write, mut read) = ws.split();

        let msg = serde_json::json!({
            "prompt": prompt,
            "session_id": sid,
        });

        if let Err(e) = write
            .send(tungstenite::Message::Text(msg.to_string().into()))
            .await
        {
            let _ = tx.send(AppEvent::WsMessage(WsInbound::Error {
                error: format!("WS send failed: {e}"),
            }));
            return;
        }

        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(tungstenite::Message::Text(text)) => {
                    if let Ok(inbound) = serde_json::from_str::<WsInbound>(&text) {
                        let is_done = matches!(inbound, WsInbound::Done | WsInbound::Error { .. });
                        let _ = tx.send(AppEvent::WsMessage(inbound));
                        if is_done {
                            break;
                        }
                    }
                }
                Ok(tungstenite::Message::Close(_)) | Err(_) => {
                    let _ = tx.send(AppEvent::WsMessage(WsInbound::Done));
                    break;
                }
                _ => {}
            }
        }
    });
}

fn spawn_ws_reader(_client: &ZeniiClient, _ws_tx: tokio::sync::mpsc::UnboundedSender<AppEvent>) {
    // WebSocket connections are per-message (see send_message).
    // A persistent notification WS can be added here later.
}
