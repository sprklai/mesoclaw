pub mod adapters;
pub mod ai;
mod commands;
pub mod config;
pub mod database;
mod plugins;
pub mod services;
pub mod skills;
pub mod skills_embedded;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(plugins::logging::build().build())
        .setup(|app| {
            plugins::logging::init(app);

            // Initialize Stronghold with Argon2 password hashing
            // Resolve app local data directory for salt file
            let app_local_data_dir = app
                .path()
                .app_local_data_dir()
                .expect("could not resolve app local data path");

            // Ensure directory exists
            std::fs::create_dir_all(&app_local_data_dir)
                .expect("failed to create app local data directory");

            // Define salt path
            let salt_path = app_local_data_dir.join("salt.txt");

            // Initialize Stronghold with Argon2 password hashing
            app.handle()
                .plugin(tauri_plugin_stronghold::Builder::with_argon2(&salt_path).build())?;

            // Initialize window state plugin
            #[cfg(desktop)]
            plugins::window_state::init(app)?;

            // Initialize autostart plugin
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))?;

            // Initialize store plugin for persistent settings
            app.handle().plugin(tauri_plugin_store::Builder::new().build())?;

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
        ])
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                plugins::window_state::on_close_requested(window);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
