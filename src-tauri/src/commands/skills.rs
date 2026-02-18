//! Tauri commands for the prompt template system.
//!
//! Replaces the previous complex skill system with lightweight, filesystem-based
//! prompt templates. All IPC command names and signatures are kept stable so
//! the frontend requires no changes.
//!
//! Commands that previously required database access (settings, enable/disable,
//! auto-select) are now no-ops — all templates are always available.

use std::collections::HashMap;

use crate::prompts::{
    get_or_init_registry, SkillDefinition, SkillInfo, SkillSettings, SkillSuggestion,
    SkillUserConfig,
};

/// List all available prompt templates as `SkillInfo` objects.
#[tauri::command]
pub async fn list_available_skills_command() -> Result<Vec<SkillInfo>, String> {
    let registry = get_or_init_registry().await;
    Ok(registry.skill_infos().await)
}

/// Get detailed information (full `SkillDefinition`) for a specific template.
#[tauri::command]
pub async fn get_skill_details_command(skill_id: String) -> Result<SkillDefinition, String> {
    let registry = get_or_init_registry().await;
    registry
        .get(&skill_id)
        .await
        .ok_or_else(|| format!("Template not found: {skill_id}"))
}

/// Get skill settings for a workspace.
///
/// In the template system every template is always enabled, so this returns a
/// synthetic `SkillSettings` with all templates marked enabled.
#[tauri::command]
pub async fn get_skill_settings_command(
    _workspace_id: String,
) -> Result<SkillSettings, String> {
    let registry = get_or_init_registry().await;
    let skills = registry
        .skill_infos()
        .await
        .into_iter()
        .map(|info| SkillUserConfig {
            skill_id: info.id,
            enabled: true,
            priority_override: None,
        })
        .collect();
    Ok(SkillSettings {
        auto_select: false,
        skills,
    })
}

/// Enable or disable a skill for a workspace (no-op in the template system).
#[tauri::command]
pub async fn set_skill_enabled_command(
    _workspace_id: String,
    _skill_id: String,
    _enabled: bool,
) -> Result<(), String> {
    Ok(())
}

/// Update skill configuration for a workspace (no-op in the template system).
#[tauri::command]
pub async fn update_skill_config_command(
    _workspace_id: String,
    _skill_id: String,
    _enabled: bool,
    _priority_override: Option<i32>,
) -> Result<(), String> {
    Ok(())
}

/// Initialise default skill settings for a workspace (no-op in the template system).
#[tauri::command]
pub async fn initialize_skill_defaults_command(_workspace_id: String) -> Result<(), String> {
    Ok(())
}

/// Reload templates from the filesystem.
#[tauri::command]
pub async fn reload_skills_command() -> Result<(), String> {
    let registry = get_or_init_registry().await;
    registry.load().await;
    Ok(())
}

/// List templates grouped by category.
#[tauri::command]
pub async fn list_skills_by_category_command(
) -> Result<HashMap<String, Vec<SkillInfo>>, String> {
    let registry = get_or_init_registry().await;
    Ok(registry.by_category().await)
}

/// Toggle auto-select mode for a workspace (no-op in the template system).
#[tauri::command]
pub async fn set_skill_auto_select_command(
    _workspace_id: String,
    _auto_select: bool,
) -> Result<(), String> {
    Ok(())
}

/// Suggest skills for a given request.
///
/// The template system does not perform AI-based selection, so this always
/// returns an empty list.
#[tauri::command]
pub async fn suggest_skills_command(
    _workspace_id: String,
    _request: String,
    _database_type: Option<String>,
) -> Result<Vec<SkillSuggestion>, String> {
    Ok(vec![])
}
