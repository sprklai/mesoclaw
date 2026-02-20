use diesel::prelude::*;

use crate::database::DbError;
use crate::database::models::settings::{Settings, SettingsChangeset, SettingsRow, SettingsUpdate};
use crate::database::utils::bool_to_int;

/// Get the current settings from the database
pub fn get_settings(conn: &mut diesel::SqliteConnection) -> Result<Settings, DbError> {
    use crate::database::schema::settings::dsl::*;

    let row = settings
        .find(1)
        .select(SettingsRow::as_select())
        .first(conn)?;

    Settings::from_row(row)
}

/// Update settings with partial data and return the updated settings
pub fn update_settings(
    conn: &mut diesel::SqliteConnection,
    update: SettingsUpdate,
) -> Result<Settings, DbError> {
    use crate::database::schema::settings::dsl::*;

    let changeset: SettingsChangeset = update.into();

    diesel::update(settings.find(1))
        .set(&changeset)
        .execute(conn)?;

    get_settings(conn)
}

/// Update the skill auto-select setting
pub fn update_skill_auto_select(
    conn: &mut diesel::SqliteConnection,
    enabled: bool,
) -> Result<Settings, DbError> {
    use crate::database::schema::settings::dsl::*;

    diesel::update(settings.find(1))
        .set(skill_auto_select.eq(bool_to_int(enabled)))
        .execute(conn)?;

    get_settings(conn)
}

/// Update a single skill's enabled state
pub fn update_skill_enabled(
    conn: &mut diesel::SqliteConnection,
    skill_id: &str,
    enabled: bool,
) -> Result<Settings, DbError> {
    use crate::database::schema::settings::dsl::*;

    // Get current settings to access the enabled list
    let current = get_settings(conn)?;
    let mut enabled_ids = current.skill_enabled_ids;

    if enabled {
        // Add skill_id if not already present
        if !enabled_ids.contains(&skill_id.to_string()) {
            enabled_ids.push(skill_id.to_string());
        }
    } else {
        // Remove skill_id if present
        enabled_ids.retain(|skill| skill != skill_id);
    }

    // Serialize back to JSON and update
    let json_ids = serde_json::to_string(&enabled_ids).unwrap_or_else(|_| "[]".to_string());

    diesel::update(settings.find(1))
        .set(skill_enabled_ids.eq(json_ids))
        .execute(conn)?;

    get_settings(conn)
}
