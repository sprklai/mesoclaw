use tauri::{State, tray::TrayIcon};

use crate::database::DbPool;
use crate::database::models::settings::{Settings, SettingsUpdate};
use crate::services::settings::{get_settings, update_settings};

/// Get the current application settings
#[tauri::command]
pub fn get_app_settings(pool: State<'_, DbPool>) -> Result<Settings, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    get_settings(&mut conn).map_err(|e| e.to_string())
}

/// Update application settings with partial data
#[tauri::command]
pub fn update_app_settings(
    pool: State<'_, DbPool>,
    update: SettingsUpdate,
) -> Result<Settings, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    update_settings(&mut conn, update).map_err(|e| e.to_string())
}

/// Set the system tray icon visibility
#[tauri::command]
pub fn set_tray_visible(tray: State<'_, TrayIcon>, visible: bool) -> Result<(), String> {
    tray.set_visible(visible).map_err(|e| e.to_string())
}

/// User identity data returned by get_user_identity_command
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserIdentity {
    pub user_name: Option<String>,
    pub app_display_name: Option<String>,
}

/// Get the user identity fields (user_name and app_display_name)
#[tauri::command]
pub fn get_user_identity_command(pool: State<'_, DbPool>) -> Result<UserIdentity, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let settings = get_settings(&mut conn).map_err(|e| e.to_string())?;
    Ok(UserIdentity {
        user_name: settings.user_name,
        app_display_name: settings.app_display_name,
    })
}

/// Set the user identity fields (user_name and app_display_name)
#[tauri::command]
pub fn set_user_identity_command(
    pool: State<'_, DbPool>,
    user_name: Option<String>,
    app_display_name: Option<String>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let update = SettingsUpdate {
        user_name: Some(user_name),
        app_display_name: Some(app_display_name),
        ..Default::default()
    };
    update_settings(&mut conn, update).map_err(|e| e.to_string())?;
    Ok(())
}
