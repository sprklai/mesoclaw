pub mod adapters;
pub mod agent;
pub mod ai;
mod commands;
pub mod config;
pub mod database;
pub mod event_bus;
pub mod gateway;
pub mod identity;
pub mod memory;
pub mod modules;
pub mod scheduler;
mod plugins;
pub mod security;
pub mod services;
pub mod tools;
pub mod prompts;

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
    pub use crate::scheduler::{JobPayload, Scheduler, Schedule};

    // Configuration
    pub use crate::config::{AppConfig, load_default_config};

    // Identity
    pub use crate::identity::loader::IdentityLoader;
}

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(plugins::logging::build().build())
        .setup(|app| {
            plugins::logging::init(app);

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
            app.manage(bus);

            // Initialize security policy and tool registry.
            let policy = Arc::new(security::SecurityPolicy::default_policy());
            let mut registry = tools::ToolRegistry::new();
            tools::register_builtin_tools(&mut registry, policy.clone());
            app.manage(policy);
            app.manage(registry);

            // Start the HTTP gateway daemon (when compiled with the gateway feature).
            #[cfg(feature = "gateway")]
            {
                let bus_for_gateway: Arc<dyn event_bus::EventBus> = app
                    .try_state::<Arc<dyn event_bus::EventBus>>()
                    .map(|s| s.inner().clone())
                    .ok_or("EventBus not initialised before gateway")?;
                tokio::spawn(async move {
                    if let Err(e) = gateway::start_gateway(bus_for_gateway).await {
                        log::error!("Gateway error: {e}");
                    }
                });
            }

            // Initialize identity loader with hot-reload watcher.
            let identity_dir = identity::default_identity_dir()?;
            let bus_ref: Arc<dyn event_bus::EventBus> = app
                .try_state::<Arc<dyn event_bus::EventBus>>()
                .map(|s| s.inner().clone())
                .ok_or("EventBus not initialised before IdentityLoader")?;
            let id_loader = identity::IdentityLoader::new_with_watcher(identity_dir, bus_ref)
                .map_err(|e| format!("identity loader: {e}"))?;
            app.manage(id_loader);

            // Initialize database and manage the connection pool
            let pool = database::init(app.handle())?;
            app.manage(pool.clone());

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
            // Approval command
            commands::approval::approve_action_command,
            // Identity commands
            identity::commands::get_identity_file_command,
            identity::commands::update_identity_file_command,
            identity::commands::list_identity_files_command,
            identity::commands::get_system_prompt_command,
            // Agent session commands
            agent::agent_commands::start_agent_session_command,
            agent::agent_commands::cancel_agent_session_command,
        ])
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                plugins::window_state::on_close_requested(window);
            }
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("error while running tauri application: {e}"));
}
