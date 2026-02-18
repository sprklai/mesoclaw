use tauri::State;

use crate::{database::DbPool, services::settings::get_settings};

/// Check if notifications are enabled in settings
#[tauri::command]
pub fn are_notifications_enabled(pool: State<DbPool>) -> Result<bool, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let settings = get_settings(&mut conn).map_err(|e| e.to_string())?;
    Ok(settings.enable_notifications)
}
