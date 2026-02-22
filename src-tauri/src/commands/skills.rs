//! Tauri commands for the prompt template system.
//!
//! Provides filesystem-based prompt templates with database-backed user preferences
//! for enable/disable and auto-select states. All IPC command names are stable so
//! the frontend requires no changes.

use std::collections::{HashMap, HashSet};

use tauri::State;

use crate::database::DbPool;
use crate::prompts::{
    SkillDefinition, SkillInfo, SkillSettings, SkillSuggestion, SkillUserConfig,
    get_or_init_registry,
};
use crate::services::settings;

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

/// Get skill settings from the database.
///
/// Returns `SkillSettings` with each skill's enabled state read from
/// the database. Skills not in the enabled list are marked as disabled.
#[tauri::command]
pub async fn get_skill_settings_command(state: State<'_, DbPool>) -> Result<SkillSettings, String> {
    let mut conn = state
        .get()
        .map_err(|e| format!("Failed to get database connection: {e}"))?;

    let app_settings =
        settings::get_settings(&mut conn).map_err(|e| format!("Failed to get settings: {e}"))?;

    let registry = get_or_init_registry().await;
    let all_skills = registry.skill_infos().await;

    let enabled_set: HashSet<String> = app_settings.skill_enabled_ids.into_iter().collect();

    let skills: Vec<SkillUserConfig> = all_skills
        .into_iter()
        .map(|info| {
            let skill_id = info.id.clone();
            SkillUserConfig {
                skill_id: info.id,
                enabled: enabled_set.contains(&skill_id),
                priority_override: None,
            }
        })
        .collect();

    Ok(SkillSettings {
        auto_select: app_settings.skill_auto_select,
        skills,
    })
}

/// Enable or disable a skill.
///
/// Persists the enabled state to the database.
#[tauri::command]
pub async fn set_skill_enabled_command(
    skill_id: String,
    enabled: bool,
    state: State<'_, DbPool>,
) -> Result<(), String> {
    let mut conn = state
        .get()
        .map_err(|e| format!("Failed to get database connection: {e}"))?;

    settings::update_skill_enabled(&mut conn, &skill_id, enabled)
        .map_err(|e| format!("Failed to update skill enabled state: {e}"))?;

    Ok(())
}

/// Update skill configuration.
///
/// Persists the enabled state to the database. Priority override is not
/// currently supported.
#[tauri::command]
pub async fn update_skill_config_command(
    skill_id: String,
    enabled: bool,
    _priority_override: Option<i32>,
    state: State<'_, DbPool>,
) -> Result<(), String> {
    let mut conn = state
        .get()
        .map_err(|e| format!("Failed to get database connection: {e}"))?;

    settings::update_skill_enabled(&mut conn, &skill_id, enabled)
        .map_err(|e| format!("Failed to update skill config: {e}"))?;

    Ok(())
}

/// Initialise default skill settings.
///
/// Not needed - defaults are derived from the template files themselves
/// and the database defaults. Returns success for backwards compatibility.
#[tauri::command]
pub async fn initialize_skill_defaults_command() -> Result<(), String> {
    // No-op: defaults are handled by database defaults and template files
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
pub async fn list_skills_by_category_command() -> Result<HashMap<String, Vec<SkillInfo>>, String> {
    let registry = get_or_init_registry().await;
    Ok(registry.by_category().await)
}

/// Toggle auto-select mode.
///
/// Persists the auto-select state to the database.
#[tauri::command]
pub async fn set_skill_auto_select_command(
    auto_select: bool,
    state: State<'_, DbPool>,
) -> Result<(), String> {
    let mut conn = state
        .get()
        .map_err(|e| format!("Failed to get database connection: {e}"))?;

    settings::update_skill_auto_select(&mut conn, auto_select)
        .map_err(|e| format!("Failed to update auto-select state: {e}"))?;

    Ok(())
}

/// Create a new skill template file on disk and reload the registry.
#[tauri::command]
pub async fn create_skill_command(
    id: String,
    name: String,
    description: String,
    category: String,
    template: String,
) -> Result<SkillInfo, String> {
    crate::prompts::loader::create_skill_file(&id, &name, &description, &category, &template)?;

    // Reload the registry
    let registry = get_or_init_registry().await;
    registry.load().await;

    // Return the newly created skill info
    registry
        .get(&id)
        .await
        .map(|t| SkillInfo {
            id: t.id,
            name: t.name,
            description: t.description,
            category: t.category,
            default_enabled: true,
            source: "filesystem".to_string(),
        })
        .ok_or_else(|| "Failed to load created skill".to_string())
}

/// Delete a skill template file from disk and reload the registry.
#[tauri::command]
pub async fn delete_skill_command(skill_id: String) -> Result<(), String> {
    let registry = get_or_init_registry().await;
    let template = registry
        .get(&skill_id)
        .await
        .ok_or_else(|| format!("Skill not found: {skill_id}"))?;
    if template.file_path.is_empty() {
        return Err(format!("No file path for skill: {skill_id}"));
    }
    std::fs::remove_file(&template.file_path)
        .map_err(|e| format!("Failed to delete skill file: {e}"))?;
    registry.load().await;
    Ok(())
}

/// Update the content of a skill template file on disk and reload the registry.
#[tauri::command]
pub async fn update_skill_command(skill_id: String, content: String) -> Result<(), String> {
    let registry = get_or_init_registry().await;
    let template = registry
        .get(&skill_id)
        .await
        .ok_or_else(|| format!("Skill not found: {skill_id}"))?;
    if template.file_path.is_empty() {
        return Err(format!("No file path for skill: {skill_id}"));
    }
    std::fs::write(&template.file_path, content.as_bytes())
        .map_err(|e| format!("Failed to write skill file: {e}"))?;
    registry.load().await;
    Ok(())
}

/// Suggest skills relevant to a given request using keyword matching.
///
/// Scores each template by the fraction of request words that appear in its
/// name, description, or category. Templates with no matching words are
/// excluded. Results are ordered by descending relevance score.
#[tauri::command]
pub async fn suggest_skills_command(request: String) -> Result<Vec<SkillSuggestion>, String> {
    let registry = get_or_init_registry().await;
    let infos = registry.skill_infos().await;

    let request_lower = request.to_lowercase();
    let words: Vec<&str> = request_lower.split_whitespace().collect();
    if words.is_empty() {
        return Ok(vec![]);
    }

    let mut suggestions: Vec<SkillSuggestion> = infos
        .into_iter()
        .filter_map(|info| {
            let haystack = format!(
                "{} {} {}",
                info.name.to_lowercase(),
                info.description.to_lowercase(),
                info.category.to_lowercase()
            );
            let matched_triggers: Vec<String> = words
                .iter()
                .filter(|&&w| haystack.contains(w))
                .map(|w| (*w).to_string())
                .collect();

            if matched_triggers.is_empty() {
                return None;
            }

            let relevance_score = matched_triggers.len() as f32 / words.len() as f32;
            Some(SkillSuggestion {
                skill_id: info.id,
                skill_name: info.name,
                description: info.description,
                relevance_score,
                matched_triggers,
            })
        })
        .collect();

    suggestions.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(suggestions)
}
