use std::sync::Arc;

use tauri::State;

use super::{
    loader::IdentityLoader,
    types::{IDENTITY_FILES, IdentityFileInfo},
};

/// Return the raw content of one identity file (e.g. `"SOUL.md"`).
#[tauri::command]
pub async fn get_identity_file_command(
    file_name: String,
    loader: State<'_, Arc<IdentityLoader>>,
) -> Result<String, String> {
    loader.get_file(&file_name)
}

/// Overwrite one identity file on disk and hot-reload.
#[tauri::command]
pub async fn update_identity_file_command(
    file_name: String,
    content: String,
    loader: State<'_, Arc<IdentityLoader>>,
) -> Result<(), String> {
    loader.update_file(&file_name, &content)
}

/// List all canonical identity files with display metadata.
#[tauri::command]
pub async fn list_identity_files_command(
    loader: State<'_, Arc<IdentityLoader>>,
) -> Result<Vec<IdentityFileInfo>, String> {
    let identity = loader.get();
    let _ = identity; // ensure loaded (no-op, just a health check)

    Ok(IDENTITY_FILES
        .iter()
        .map(|(file_name, description)| IdentityFileInfo {
            name: file_name.trim_end_matches(".md").to_string(),
            file_name: file_name.to_string(),
            description: description.to_string(),
        })
        .collect())
}

/// Return the fully assembled system prompt.
#[tauri::command]
pub async fn get_system_prompt_command(
    loader: State<'_, Arc<IdentityLoader>>,
) -> Result<String, String> {
    Ok(loader.build_system_prompt())
}
