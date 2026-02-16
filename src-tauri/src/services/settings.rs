use diesel::prelude::*;

use crate::database::DbError;
use crate::database::models::settings::{Settings, SettingsChangeset, SettingsRow, SettingsUpdate};

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
