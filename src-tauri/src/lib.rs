pub mod activity;
pub mod adapters;
pub mod agent;
pub mod agents;
pub mod ai;
pub mod channels;
mod commands;
pub mod config;
pub mod database;
pub mod event_bus;
#[cfg(feature = "wasm-ext")]
pub mod extensions;
pub mod gateway;
pub mod identity;
pub mod memory;
pub mod modules;
mod plugins;
pub mod prompts;
pub mod scheduler;
pub mod security;
pub mod services;
pub mod tools;

/// Convenience re-exports for the most commonly used traits and types across
/// the MesoClaw codebase.
///
/// ```rust
/// use mesoclaw::prelude::*;
/// ```
pub mod prelude {
    // AI provider trait
    pub use crate::ai::provider::LLMProvider;
    pub use crate::ai::providers::{CostTier, ModelRouter, ModelTarget, TaskType};
    pub use crate::ai::types::{CompletionRequest, CompletionResponse, Message, MessageRole};

    // Tool system
    pub use crate::tools::{Tool, ToolRegistry, ToolResult};

    // Memory subsystem
    pub use crate::memory::traits::{Memory, MemoryCategory, MemoryEntry};

    // Security
    pub use crate::security::{AutonomyLevel, SecurityPolicy, ValidationResult};

    // Event bus
    pub use crate::event_bus::{AppEvent, EventBus, TokioBroadcastBus};

    // Scheduler
    pub use crate::scheduler::{JobPayload, Schedule, Scheduler};

    // Configuration
    pub use crate::config::{AppConfig, load_default_config};

    // Identity
    pub use crate::identity::loader::IdentityLoader;

    // Channel abstraction
    pub use crate::channels::{Channel, ChannelManager, ChannelMessage, TauriIpcChannel};
}

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Install the ring crypto provider for rustls before any network I/O.
    // Multiple optional dependencies (serenity, reqwest, slack-morphism, teloxide)
    // all pull in rustls; without an explicit default, rustls panics at runtime.
    let _ = rustls::crypto::ring::default_provider().install_default();

    plugins::logging::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {

            // Initialize Stronghold with Argon2 password hashing
            // Resolve app local data directory for salt file
            let app_local_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|e| format!("could not resolve app local data path: {e}"))?;

            // Ensure directory exists
            std::fs::create_dir_all(&app_local_data_dir)
                .map_err(|e| format!("failed to create app local data directory: {e}"))?;

            // Define salt path
            let salt_path = app_local_data_dir.join("salt.txt");

            // Initialize Stronghold with Argon2 password hashing
            app.handle()
                .plugin(tauri_plugin_stronghold::Builder::with_argon2(&salt_path).build())?;

            // Initialize window state plugin
            #[cfg(desktop)]
            plugins::window_state::init(app)?;

            // Initialize single-instance guard: prevent a second desktop process
            // from starting while the daemon is already running.
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
                // When a second instance is launched, focus the existing window.
                log::info!("single-instance: second launch detected, ignoring");
            }))?;

            // Initialize deep-link handler for OAuth callback URIs
            // (e.g. mesoclaw://oauth/callback?code=...).
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_deep_link::init())?;

            // Initialize autostart plugin
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))?;

            // Initialize store plugin for persistent settings
            app.handle().plugin(tauri_plugin_store::Builder::new().build())?;

            // Initialize and register the application event bus.
            let bus: Arc<dyn event_bus::EventBus> =
                Arc::new(event_bus::TokioBroadcastBus::new());
            event_bus::TauriBridge::new(bus.clone(), app.handle().clone()).start();
            app.manage(bus.clone());

            // Initialize activity buffer and subscribe to event bus.
            let activity_buffer = Arc::new(activity::ActivityBuffer::with_default_size());
            activity::ActivityBuffer::subscribe_to_bus(Arc::clone(&activity_buffer), bus);
            app.manage(activity_buffer);

            // Initialize security policy and tool registry.
            let policy = Arc::new(security::SecurityPolicy::default_policy());
            let mut registry = tools::ToolRegistry::new();
            tools::register_builtin_tools(&mut registry, policy.clone());
            app.manage(policy);
            // Store registry in an Arc so agent_commands can clone a cheap handle.
            app.manage(Arc::new(registry));

            // Initialize session cancellation map for agent_commands.
            let cancel_map: agent::agent_commands::SessionCancelMap =
                Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
            app.manage(cancel_map);

            // Initialize session router for agent session management.
            let session_router = Arc::new(agent::session_router::SessionRouter::new());
            app.manage(session_router);

            // Initialize and manage the in-memory store so IPC commands can reach it.
            app.manage(Arc::new(memory::store::InMemoryStore::new_mock()));

            // Start the HTTP gateway daemon (when compiled with the gateway feature).
            // NOTE: The gateway spawn is deferred until after database and identity
            // loader initialisation so that DbPool and IdentityLoader are available.
            // See the `#[cfg(feature = "gateway")]` block further below.

            // Initialize identity loader with hot-reload watcher.
            let identity_dir = identity::default_identity_dir()?;
            let bus_ref: Arc<dyn event_bus::EventBus> = app
                .try_state::<Arc<dyn event_bus::EventBus>>()
                .map(|s| s.inner().clone())
                .ok_or("EventBus not initialised before IdentityLoader")?;
            let id_loader = identity::IdentityLoader::new_with_watcher(identity_dir, bus_ref)
                .map_err(|e| format!("identity loader: {e}"))?;
            app.manage(id_loader);

            // Run the boot sequence.  Creates ~/.mesoclaw/ dirs, loads config,
            // starts the scheduler, starts channels, and emits SystemReady.
            {
                let bus_boot: Arc<dyn event_bus::EventBus> = app
                    .try_state::<Arc<dyn event_bus::EventBus>>()
                    .map(|s| s.inner().clone())
                    .ok_or("EventBus not initialised before BootSequence")?;
                let channel_mgr = Arc::new(channels::ChannelManager::new());
                // Expose ChannelManager to Tauri IPC commands.
                app.manage(Arc::clone(&channel_mgr));
                // Register the Tauri IPC channel so desktop events flow into the agent loop.
                let ipc_ch = Arc::new(channels::TauriIpcChannel::new(Arc::clone(&bus_boot)));
                let channel_mgr_clone = Arc::clone(&channel_mgr);
                let bus_boot_clone = Arc::clone(&bus_boot);
                let bus_for_router = Arc::clone(&bus_boot);
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = channel_mgr_clone.register(ipc_ch).await {
                        log::warn!("boot: failed to register tauri-ipc channel: {e}");
                    }
                    // Register Telegram channel from keyring on startup (if token exists).
                    #[cfg(feature = "channels-telegram")]
                    {
                        use crate::config::app_identity::KEYCHAIN_SERVICE;
                        match keyring::Entry::new(KEYCHAIN_SERVICE, "channel:telegram:token") {
                            Ok(entry) => {
                                if let Ok(token) = entry.get_password() {
                                    let allowed_ids: Vec<i64> =
                                        keyring::Entry::new(KEYCHAIN_SERVICE, "channel:telegram:allowed_chat_ids")
                                            .ok()
                                            .and_then(|e| e.get_password().ok())
                                            .unwrap_or_default()
                                            .split(',')
                                            .filter_map(|s| s.trim().parse().ok())
                                            .collect();
                                    let timeout: u32 =
                                        keyring::Entry::new(KEYCHAIN_SERVICE, "channel:telegram:polling_timeout_secs")
                                            .ok()
                                            .and_then(|e| e.get_password().ok())
                                            .and_then(|s| s.parse().ok())
                                            .unwrap_or(30);
                                    let mut config =
                                        channels::TelegramConfig::with_allowed_ids(token, allowed_ids);
                                    config.polling_timeout_secs = timeout;
                                    let telegram = Arc::new(channels::TelegramChannel::new(config));
                                    if let Err(e) = channel_mgr_clone.register(telegram).await {
                                        log::warn!("boot: telegram channel registration failed: {e}");
                                    } else {
                                        log::info!("boot: telegram channel registered from keyring");
                                    }
                                } else {
                                    log::info!("boot: no telegram bot token in keyring, channel not started");
                                }
                            }
                            Err(e) => log::warn!("boot: keyring access failed for telegram: {e}"),
                        }
                    }
                    // Register Discord channel from keyring on startup (if token exists).
                    #[cfg(feature = "channels-discord")]
                    {
                        use crate::config::app_identity::KEYCHAIN_SERVICE;
                        match keyring::Entry::new(KEYCHAIN_SERVICE, "channel:discord:token") {
                            Ok(entry) => {
                                if let Ok(token) = entry.get_password() {
                                    if !token.is_empty() {
                                        let allowed_guild_ids: Vec<u64> =
                                            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:discord:allowed_guild_ids")
                                                .ok()
                                                .and_then(|e| e.get_password().ok())
                                                .unwrap_or_default()
                                                .split(',')
                                                .filter_map(|s| s.trim().parse().ok())
                                                .collect();
                                        let allowed_channel_ids: Vec<u64> =
                                            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:discord:allowed_channel_ids")
                                                .ok()
                                                .and_then(|e| e.get_password().ok())
                                                .unwrap_or_default()
                                                .split(',')
                                                .filter_map(|s| s.trim().parse().ok())
                                                .collect();
                                        let config = channels::DiscordConfig::with_allowlists(
                                            token,
                                            allowed_guild_ids,
                                            allowed_channel_ids,
                                        );
                                        let discord = Arc::new(channels::DiscordChannel::new(config));
                                        if let Err(e) = channel_mgr_clone.register(discord).await {
                                            log::warn!("boot: discord channel registration failed: {e}");
                                        } else {
                                            log::info!("boot: discord channel registered from keyring");
                                        }
                                    } else {
                                        log::info!("boot: no discord bot token in keyring, channel not started");
                                    }
                                } else {
                                    log::info!("boot: no discord bot token in keyring, channel not started");
                                }
                            }
                            Err(e) => log::warn!("boot: keyring access failed for discord: {e}"),
                        }
                    }

                    // Register Matrix channel from keyring on startup (if credentials exist).
                    #[cfg(feature = "channels-matrix")]
                    {
                        use crate::config::app_identity::KEYCHAIN_SERVICE;
                        let homeserver = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:homeserver_url")
                            .ok()
                            .and_then(|e| e.get_password().ok())
                            .unwrap_or_default();
                        let username = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:username")
                            .ok()
                            .and_then(|e| e.get_password().ok())
                            .unwrap_or_default();
                        let access_token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:access_token")
                            .ok()
                            .and_then(|e| e.get_password().ok())
                            .unwrap_or_default();
                        if !access_token.is_empty() && !homeserver.is_empty() {
                            let allowed_room_ids: Vec<String> =
                                keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:allowed_room_ids")
                                    .ok()
                                    .and_then(|e| e.get_password().ok())
                                    .unwrap_or_default()
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                    .map(String::from)
                                    .collect();
                            let config = channels::MatrixConfig::with_allowed_rooms(
                                homeserver,
                                username,
                                access_token,
                                allowed_room_ids,
                            );
                            let matrix = Arc::new(channels::MatrixChannel::new(config));
                            if let Err(e) = channel_mgr_clone.register(matrix).await {
                                log::warn!("boot: matrix channel registration failed: {e}");
                            } else {
                                log::info!("boot: matrix channel registered from keyring");
                            }
                        } else {
                            log::info!("boot: no matrix credentials in keyring, channel not started");
                        }
                    }

                    // Register Slack channel from keyring on startup (if tokens exist).
                    #[cfg(feature = "channels-slack")]
                    {
                        use crate::config::app_identity::KEYCHAIN_SERVICE;
                        let bot_token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:slack:bot_token")
                            .ok()
                            .and_then(|e| e.get_password().ok())
                            .unwrap_or_default();
                        let app_token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:slack:app_token")
                            .ok()
                            .and_then(|e| e.get_password().ok())
                            .unwrap_or_default();
                        if !bot_token.is_empty() && !app_token.is_empty() {
                            let allowed_channel_ids: Vec<String> =
                                keyring::Entry::new(KEYCHAIN_SERVICE, "channel:slack:allowed_channel_ids")
                                    .ok()
                                    .and_then(|e| e.get_password().ok())
                                    .unwrap_or_default()
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                    .map(String::from)
                                    .collect();
                            let config = channels::SlackConfig::with_allowed_channels(
                                bot_token,
                                app_token,
                                allowed_channel_ids,
                            );
                            let slack = Arc::new(channels::SlackChannel::new(config));
                            if let Err(e) = channel_mgr_clone.register(slack).await {
                                log::warn!("boot: slack channel registration failed: {e}");
                            } else {
                                log::info!("boot: slack channel registered from keyring");
                            }
                        } else {
                            log::info!("boot: no slack tokens in keyring, channel not started");
                        }
                    }

                    match services::boot::BootSequence::new(bus_boot_clone, channel_mgr_clone) {
                        Ok(seq) => match seq.run().await {
                            Ok(ctx) => {
                                log::info!("boot: sequence complete; {} channel handle(s)", ctx.channel_handles.len());
                                // Spawn channel router: forward all inbound channel messages to EventBus.
                                let mut msg_rx = ctx.message_rx;
                                let bus_router = bus_for_router;
                                tauri::async_runtime::spawn(async move {
                                    while let Some(msg) = msg_rx.recv().await {
                                        let event = event_bus::AppEvent::ChannelMessage {
                                            channel: msg.channel.clone(),
                                            from: msg.sender.unwrap_or_default(),
                                            content: msg.content.clone(),
                                        };
                                        if let Err(e) = bus_router.publish(event) {
                                            log::warn!("channel-router: publish error: {e}");
                                        }
                                    }
                                    log::info!("channel-router: receiver closed, task exiting");
                                });
                            }
                            Err(e) => log::error!("boot: sequence failed: {e}"),
                        },
                        Err(e) => log::error!("boot: could not create BootSequence: {e}"),
                    }
                });
            }

            // Initialize database and manage the connection pool
            let pool = database::init(app.handle())?;
            app.manage(pool.clone());

            // Initialize scheduler before the gateway so it can be threaded in.
            // Initialised with SQLite persistence + agent loop support.
            let sched = {
                use scheduler::traits::Scheduler as _;
                let bus_sched: Arc<dyn event_bus::EventBus> = app
                    .try_state::<Arc<dyn event_bus::EventBus>>()
                    .map(|s| s.inner().clone())
                    .ok_or("EventBus not initialised before scheduler")?;
                let policy_sched = app
                    .try_state::<Arc<security::SecurityPolicy>>()
                    .map(|s| s.inner().clone())
                    .ok_or("SecurityPolicy not initialised before scheduler")?;
                let registry_sched = app
                    .try_state::<Arc<tools::ToolRegistry>>()
                    .map(|s| s.inner().clone())
                    .ok_or("ToolRegistry not initialised before scheduler")?;
                let id_loader_sched = app
                    .try_state::<Arc<identity::IdentityLoader>>()
                    .map(|s| s.inner().clone())
                    .ok_or("IdentityLoader not initialised before scheduler")?;

                // Try to resolve the active LLM provider; scheduler still works without one.
                let agent_components = agent::agent_commands::resolve_active_provider(&pool)
                    .ok()
                    .map(|provider| scheduler::tokio_scheduler::AgentComponents {
                        provider,
                        tool_registry: registry_sched,
                        security_policy: policy_sched,
                        identity_loader: id_loader_sched,
                    });

                let s = if let Some(components) = agent_components {
                    scheduler::TokioScheduler::new_with_agent(bus_sched, Some(pool.clone()), components)
                } else {
                    log::warn!("scheduler: no LLM provider configured; Heartbeat/AgentTurn payloads will be skipped");
                    scheduler::TokioScheduler::new_with_persistence(bus_sched, Some(pool.clone()))
                };

                // Start the background tick loop.
                let s_start = Arc::clone(&s);
                tauri::async_runtime::spawn(async move { s_start.start().await });

                // Auto-seed a default 30-min heartbeat job if none exists.
                let s_seed = Arc::clone(&s);
                tauri::async_runtime::spawn(async move {
                    use scheduler::traits::Scheduler as _;
                    let jobs = s_seed.list_jobs().await;
                    if !jobs
                        .iter()
                        .any(|j| matches!(j.payload, scheduler::traits::JobPayload::Heartbeat))
                    {
                        s_seed
                            .add_job(scheduler::traits::ScheduledJob {
                                id: String::new(),
                                name: "Heartbeat".to_string(),
                                schedule: scheduler::traits::Schedule::Interval {
                                    secs: scheduler::DEFAULT_HEARTBEAT_INTERVAL_SECS,
                                },
                                session_target: scheduler::traits::SessionTarget::Main,
                                payload: scheduler::traits::JobPayload::Heartbeat,
                                enabled: true,
                                error_count: 0,
                                next_run: None,
                                active_hours: None,
                                delete_after_run: false,
                            })
                            .await;
                        log::info!(
                            "scheduler: seeded default heartbeat job ({} min interval)",
                            scheduler::DEFAULT_HEARTBEAT_INTERVAL_SECS / 60
                        );
                    }
                });

                s
            };
            app.manage(Arc::clone(&sched));

            // Initialize module registry — discover user-installed sidecar modules.
            let modules_dir = app_local_data_dir.join("modules");
            std::fs::create_dir_all(&modules_dir).ok();
            let mut scratch_tool_registry = tools::ToolRegistry::new();
            let module_registry = Arc::new(modules::ModuleRegistry::discover(
                &modules_dir,
                &mut scratch_tool_registry,
                Arc::new(security::SecurityPolicy::default_policy()),
                None,
            ));
            app.manage(Arc::clone(&module_registry));

            // Start the HTTP gateway daemon (deferred until DB + identity are ready).
            #[cfg(feature = "gateway")]
            {
                let bus_for_gateway: Arc<dyn event_bus::EventBus> = app
                    .try_state::<Arc<dyn event_bus::EventBus>>()
                    .map(|s| s.inner().clone())
                    .ok_or("EventBus not initialised before gateway")?;
                let sessions_for_gateway = Arc::new(agent::session_router::SessionRouter::new());
                let modules_for_gateway = Arc::clone(&module_registry);
                let pool_for_gateway = pool.clone();
                let identity_for_gateway: Arc<identity::IdentityLoader> = app
                    .try_state::<Arc<identity::IdentityLoader>>()
                    .map(|s| s.inner().clone())
                    .ok_or("IdentityLoader not initialised before gateway")?;
                let memory_for_gateway: Arc<memory::store::InMemoryStore> = app
                    .try_state::<Arc<memory::store::InMemoryStore>>()
                    .map(|s| s.inner().clone())
                    .ok_or("MemoryStore not initialised before gateway")?;
                let sched_for_gateway = Arc::clone(&sched);
                let cancel_map_for_gateway: agent::agent_commands::SessionCancelMap = app
                    .try_state::<agent::agent_commands::SessionCancelMap>()
                    .map(|s| s.inner().clone())
                    .ok_or("SessionCancelMap not initialised before gateway")?;
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = gateway::start_gateway(
                        bus_for_gateway,
                        sessions_for_gateway,
                        modules_for_gateway,
                        pool_for_gateway,
                        identity_for_gateway,
                        memory_for_gateway,
                        sched_for_gateway,
                        cancel_map_for_gateway,
                    )
                    .await
                    {
                        log::error!("Gateway error: {e}");
                    }
                });
            }

            // ── Channel-Agent Bridge ──────────────────────────────────────────────────────
            // Subscribes to ChannelMessage events and auto-triggers the agent.
            // Runs the agent loop for each inbound external-channel message and routes
            // the response back via ChannelManager.send() so Telegram users get a reply.
            {
                use agent::{
                    agent_commands::{resolve_active_provider, SessionCancelMap},
                    loop_::{AgentConfig, AgentLoop, AgentMessage},
                    session_router::SessionRouter,
                };
                use event_bus::{AppEvent, EventFilter, EventType};

                // Per-channel persistent session history (7.1.5).
                let bridge_sessions = Arc::new(SessionRouter::new());

                let bridge_bus: Arc<dyn event_bus::EventBus> = app
                    .try_state::<Arc<dyn event_bus::EventBus>>()
                    .map(|s| s.inner().clone())
                    .ok_or("EventBus not initialised before channel-bridge")?;
                let bridge_mgr: Arc<channels::ChannelManager> = app
                    .try_state::<Arc<channels::ChannelManager>>()
                    .map(|s| s.inner().clone())
                    .ok_or("ChannelManager not initialised before channel-bridge")?;
                let bridge_pool: database::DbPool = pool.clone();
                let bridge_registry: Arc<tools::ToolRegistry> = app
                    .try_state::<Arc<tools::ToolRegistry>>()
                    .map(|s| s.inner().clone())
                    .ok_or("ToolRegistry not initialised before channel-bridge")?;
                let bridge_policy: Arc<security::SecurityPolicy> = app
                    .try_state::<Arc<security::SecurityPolicy>>()
                    .map(|s| s.inner().clone())
                    .ok_or("SecurityPolicy not initialised before channel-bridge")?;
                let bridge_identity: Arc<identity::IdentityLoader> = app
                    .try_state::<Arc<identity::IdentityLoader>>()
                    .map(|s| s.inner().clone())
                    .ok_or("IdentityLoader not initialised before channel-bridge")?;
                let bridge_cancel: SessionCancelMap = app
                    .try_state::<SessionCancelMap>()
                    .map(|s| s.inner().clone())
                    .ok_or("SessionCancelMap not initialised before channel-bridge")?;

                // Move into the subscription loop.
                let sessions_for_bridge = Arc::clone(&bridge_sessions);
                tauri::async_runtime::spawn(async move {
                    use tokio::sync::broadcast::error::RecvError;
                    let mut rx = bridge_bus
                        .subscribe_filtered(EventFilter::new(vec![EventType::ChannelMessage]));

                    loop {
                        match rx.recv().await {
                            Ok(AppEvent::ChannelMessage { channel, from, content }) => {
                                // Skip the internal Tauri IPC channel — its messages are
                                // already handled by the desktop UI; routing them through
                                // the agent a second time would create a feedback loop.
                                if channel == "tauri-ipc" {
                                    continue;
                                }

                                // ── Bot command short-circuits ──────────────────────────────
                                // Commands are identified by their raw content (e.g. "/cancel").
                                // They are handled directly here and never forwarded to the
                                // agent loop, so they respond instantly regardless of provider
                                // availability.
                                let trimmed = content.trim();

                                if trimmed.eq_ignore_ascii_case("/cancel") {
                                    let session_id =
                                        format!("channel:dm:{channel}:{from}");
                                    let cancelled = bridge_cancel
                                        .lock()
                                        .ok()
                                        .and_then(|map| {
                                            map.get(&session_id).map(|flag| {
                                                flag.store(
                                                    true,
                                                    std::sync::atomic::Ordering::SeqCst,
                                                );
                                                true
                                            })
                                        })
                                        .unwrap_or(false);
                                    let reply = if cancelled {
                                        "Agent session cancelled.".to_string()
                                    } else {
                                        "No active session to cancel.".to_string()
                                    };
                                    log::info!(
                                        "channel-bridge [{channel}]: /cancel from {from} \
                                         (cancelled={cancelled})"
                                    );
                                    if let Err(e) = bridge_mgr
                                        .send(&channel, &reply, Some(&from))
                                        .await
                                    {
                                        log::warn!(
                                            "channel-bridge [{channel}]: /cancel reply \
                                             failed: {e}"
                                        );
                                    }
                                    continue;
                                }

                                if trimmed.eq_ignore_ascii_case("/status") {
                                    let session_id =
                                        format!("channel:dm:{channel}:{from}");
                                    let active = bridge_cancel
                                        .lock()
                                        .ok()
                                        .map(|map| map.contains_key(&session_id))
                                        .unwrap_or(false);
                                    let reply = if active {
                                        "Agent session is active.".to_string()
                                    } else {
                                        "No active agent session.".to_string()
                                    };
                                    log::info!(
                                        "channel-bridge [{channel}]: /status from {from} \
                                         (active={active})"
                                    );
                                    if let Err(e) = bridge_mgr
                                        .send(&channel, &reply, Some(&from))
                                        .await
                                    {
                                        log::warn!(
                                            "channel-bridge [{channel}]: /status reply \
                                             failed: {e}"
                                        );
                                    }
                                    continue;
                                }

                                // Clone everything needed for the per-message spawned task.
                                let bus = Arc::clone(&bridge_bus);
                                let mgr = Arc::clone(&bridge_mgr);
                                let pool = bridge_pool.clone();
                                let reg = Arc::clone(&bridge_registry);
                                let pol = Arc::clone(&bridge_policy);
                                let ident = Arc::clone(&bridge_identity);
                                let cmap = Arc::clone(&bridge_cancel);
                                let sessions = Arc::clone(&sessions_for_bridge);
                                let chan = channel.clone();
                                let chat_id = from.clone();

                                tauri::async_runtime::spawn(async move {
                                    let provider = match resolve_active_provider(&pool) {
                                        Ok(p) => p,
                                        Err(e) => {
                                            log::warn!(
                                                "channel-bridge [{chan}]: provider error: {e}"
                                            );
                                            return;
                                        }
                                    };

                                    // ── Session routing (7.1.5) ───────────────────────
                                    // Resolve a structured session key via SessionRouter so
                                    // Telegram DMs get per-user persistent conversation history.
                                    let session_key =
                                        sessions.resolve(&chan, Some(&chat_id));
                                    let session_id = session_key.as_str();
                                    let flag = Arc::new(
                                        std::sync::atomic::AtomicBool::new(false),
                                    );
                                    if let Ok(mut map) = cmap.lock() {
                                        map.insert(session_id.clone(), Arc::clone(&flag));
                                    }

                                    let _ = bus.publish(AppEvent::AgentStarted {
                                        session_id: session_id.clone(),
                                    });

                                    let system_prompt = ident.build_system_prompt();
                                    let agent = AgentLoop::new(
                                        provider,
                                        reg,
                                        pol,
                                        Some(bus.clone()),
                                        AgentConfig::default(),
                                    )
                                    .with_cancel_flag(Arc::clone(&flag));

                                    // Build history from persisted session messages +
                                    // the new user turn.  Keeps conversation context
                                    // across multiple messages from the same peer.
                                    // get_or_create is infallible in practice; on lock
                                    // poisoning we fall back to an empty message list.
                                    let prior_messages = sessions
                                        .get_or_create(session_key.clone())
                                        .map(|s| s.messages)
                                        .unwrap_or_default();
                                    let mut history: Vec<AgentMessage> =
                                        vec![AgentMessage::System {
                                            content: system_prompt.clone(),
                                        }];
                                    for msg in &prior_messages {
                                        match msg.role.as_str() {
                                            "user" => history.push(AgentMessage::User {
                                                content: msg.content.clone(),
                                            }),
                                            "assistant" => {
                                                history.push(AgentMessage::Assistant {
                                                    content: msg.content.clone(),
                                                    tool_calls: vec![],
                                                })
                                            }
                                            _ => {}
                                        }
                                    }
                                    history.push(AgentMessage::User {
                                        content: content.clone(),
                                    });

                                    match agent.run_with_history(&mut history).await {
                                        Ok(response) => {
                                            // Persist the new user + assistant turns.
                                            let _ = sessions.push_message(
                                                &session_key,
                                                "user",
                                                &content,
                                            );
                                            let _ = sessions.push_message(
                                                &session_key,
                                                "assistant",
                                                &response,
                                            );
                                            // Compact to prevent unbounded growth.
                                            let _ = sessions.compact(&session_key, 50);

                                            // Emit AgentComplete so TauriBridge forwards to
                                            // the desktop frontend as well.
                                            let _ = bus.publish(AppEvent::AgentComplete {
                                                session_id: session_id.clone(),
                                                message: response.clone(),
                                            });
                                            // Send response back through the originating channel.
                                            if let Err(e) =
                                                mgr.send(&chan, &response, Some(&chat_id)).await
                                            {
                                                log::warn!(
                                                    "channel-bridge [{chan}]: send failed: {e}"
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "channel-bridge [{chan}]: agent error: {e}"
                                            );
                                        }
                                    }

                                    if let Ok(mut map) = cmap.lock() {
                                        map.remove(&session_id);
                                    }
                                });
                            }
                            Ok(_) => {} // Non-ChannelMessage event passed through filter — discard.
                            Err(RecvError::Lagged(n)) => {
                                log::warn!("channel-bridge: lagged {n} events");
                            }
                            Err(RecvError::Closed) => {
                                log::info!("channel-bridge: event bus closed, exiting");
                                break;
                            }
                        }
                    }
                });
            }

            plugins::system_tray::setup(app, &pool)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Settings commands
            commands::settings::get_app_settings,
            commands::settings::update_app_settings,
            commands::settings::set_tray_visible,
            // Window commands
            commands::window::close_splashscreen,
            // Notification commands
            commands::notifications::are_notifications_enabled,
            // Keychain commands
            commands::keychain::keychain_set,
            commands::keychain::keychain_get,
            commands::keychain::keychain_delete,
            commands::keychain::keychain_exists,
            // File browser commands
            commands::file_browser::list_directory,
            commands::file_browser::get_home_directory,
            commands::file_browser::get_common_directories,
            // LLM configuration commands
            commands::llm::configure_llm_provider_command,
            commands::llm::test_llm_provider_command,
            commands::llm::get_llm_provider_config_command,
            // AI provider management commands
            commands::ai_providers::list_ai_providers_command,
            commands::ai_providers::list_providers_with_key_status_command,
            commands::ai_providers::get_provider_by_id_command,
            commands::ai_providers::test_provider_connection_command,
            commands::ai_providers::add_custom_model_command,
            commands::ai_providers::delete_model_command,
            commands::ai_providers::reactivate_provider_command,
            commands::ai_providers::update_provider_command,
            commands::ai_providers::add_user_provider_command,
            commands::ai_providers::delete_user_provider_command,
            commands::ai_providers::get_global_default_model_command,
            commands::ai_providers::set_global_default_model_command,
            // Ollama commands
            commands::ollama::discover_ollama_models_command,
            // Generic AI chat commands (database-specific chat removed)
            commands::chat::get_available_models_command,
            // Chat session persistence commands
            commands::chat_sessions::list_chat_sessions_command,
            commands::chat_sessions::get_chat_session_command,
            commands::chat_sessions::create_chat_session_command,
            commands::chat_sessions::delete_chat_session_command,
            commands::chat_sessions::load_messages_command,
            commands::chat_sessions::save_message_command,
            commands::chat_sessions::clear_session_messages_command,
            // Streaming chat command
            commands::streaming_chat::stream_chat_command,
            // Skill system commands
            commands::skills::list_available_skills_command,
            commands::skills::get_skill_details_command,
            commands::skills::get_skill_settings_command,
            commands::skills::set_skill_enabled_command,
            commands::skills::update_skill_config_command,
            commands::skills::initialize_skill_defaults_command,
            // Database-specific skill execution commands commented out
            // commands::skills::execute_with_skills_command,
            // commands::skills::execute_skill_command,
            commands::skills::reload_skills_command,
            commands::skills::list_skills_by_category_command,
            commands::skills::set_skill_auto_select_command,
            commands::skills::suggest_skills_command,
            commands::skills::delete_skill_command,
            commands::skills::update_skill_command,
            // Approval command
            commands::approval::approve_action_command,
            commands::approval::get_daemon_config_command,
            // Identity commands
            identity::commands::get_identity_file_command,
            identity::commands::update_identity_file_command,
            identity::commands::list_identity_files_command,
            identity::commands::get_system_prompt_command,
            // Agent session commands
            agent::agent_commands::start_agent_session_command,
            agent::agent_commands::cancel_agent_session_command,
            // Agent management commands
            agent::commands::list_agents_command,
            agent::commands::create_agent_command,
            agent::commands::get_agent_command,
            agent::commands::update_agent_command,
            agent::commands::delete_agent_command,
            agent::commands::run_agent_command,
            agent::commands::list_sessions_command,
            agent::commands::get_session_command,
            agent::commands::get_workspace_file_command,
            agent::commands::update_workspace_file_command,
            agent::commands::list_workspace_files_command,
            // Agent database commands
            commands::agents::list_db_agents_command,
            commands::agents::get_db_agent_command,
            commands::agents::create_db_agent_command,
            commands::agents::update_db_agent_command,
            commands::agents::delete_db_agent_command,
            commands::agents::list_db_agent_sessions_command,
            commands::agents::list_recent_db_sessions_command,
            commands::agents::list_active_db_runs_command,
            commands::agents::get_db_run_details_command,
            commands::agents::cancel_db_run_command,
            // Channel management commands
            commands::channels::start_channel_command,
            commands::channels::channel_health_command,
            commands::channels::disconnect_channel_command,
            commands::channels::test_channel_connection_command,
            commands::channels::list_channels_command,
            commands::channels::send_channel_message_command,
            // Log viewer commands
            commands::logs::get_logs_command,
            // Prompt generator commands
            commands::prompt_generator::generate_prompt_command,
            commands::prompt_generator::list_generated_prompts_command,
            commands::prompt_generator::delete_generated_prompt_command,
            commands::prompt_generator::update_generated_prompt_command,
            // Scheduler commands
            scheduler::commands::list_jobs_command,
            scheduler::commands::create_job_command,
            scheduler::commands::toggle_job_command,
            scheduler::commands::delete_job_command,
            scheduler::commands::job_history_command,
            // Module management commands
            modules::commands::list_modules_command,
            modules::commands::start_module_command,
            modules::commands::stop_module_command,
            modules::commands::create_module_command,
            // Memory commands
            memory::commands::store_memory_command,
            memory::commands::search_memory_command,
            memory::commands::forget_memory_command,
            memory::commands::get_daily_memory_command,
            // Activity commands
            activity::commands::get_recent_activity_command,
        ])
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                plugins::window_state::on_close_requested(window);
            }
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log::error!("Fatal: failed to run tauri application: {e}");
            std::process::exit(1);
        });
}
