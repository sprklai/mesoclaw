//! Tauri commands for the prompt template system.
//!
//! Replaces the previous complex skill system with lightweight, filesystem-based
//! prompt templates. All IPC command names are kept stable so the frontend
//! requires no changes.
//!
//! In the template system every template is always available and enabled.
//! Write commands that previously modified per-skill settings (enable/disable,
//! auto-select, priority) are not supported and return an explicit error so
//! callers can gate their UX accordingly instead of receiving a silent no-op.

use std::collections::HashMap;

use crate::prompts::{
    SkillDefinition, SkillInfo, SkillSettings, SkillSuggestion, SkillUserConfig,
    get_or_init_registry,
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

/// Get skill settings.
///
/// In the template system every template is always enabled, so this returns a
/// synthetic `SkillSettings` with all templates marked enabled.
#[tauri::command]
pub async fn get_skill_settings_command() -> Result<SkillSettings, String> {
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

/// Enable or disable a skill.
///
/// Not supported in the filesystem-based template system — all templates are
/// always available. Returns an explicit error so the caller can gate its UI
/// rather than silently discarding the write.
#[tauri::command]
pub async fn set_skill_enabled_command(_skill_id: String, _enabled: bool) -> Result<(), String> {
    Err(
        "not supported: template system does not persist per-skill enable/disable state"
            .to_string(),
    )
}

/// Update skill configuration.
///
/// Not supported in the filesystem-based template system. Returns an explicit
/// error so the caller can gate its UI rather than silently discarding the write.
#[tauri::command]
pub async fn update_skill_config_command(
    _skill_id: String,
    _enabled: bool,
    _priority_override: Option<i32>,
) -> Result<(), String> {
    Err("not supported: template system does not persist per-skill configuration".to_string())
}

/// Initialise default skill settings.
///
/// Not supported in the filesystem-based template system — defaults are derived
/// from the template files themselves. Returns an explicit error so the caller
/// can gate its UI rather than silently discarding the operation.
#[tauri::command]
pub async fn initialize_skill_defaults_command() -> Result<(), String> {
    Err("not supported: template system does not use persisted skill defaults".to_string())
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
/// Not supported in the filesystem-based template system. Returns an explicit
/// error so the caller can gate its UI rather than silently discarding the write.
#[tauri::command]
pub async fn set_skill_auto_select_command(_auto_select: bool) -> Result<(), String> {
    Err("not supported: template system does not persist auto-select state".to_string())
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
