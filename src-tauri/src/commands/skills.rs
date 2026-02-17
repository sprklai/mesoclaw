//! Tauri commands for the skill system.
//!
//! These commands expose skill functionality to the frontend.

use tauri::State;

// Database-specific imports commented out
// use crate::adapters::KnoAdapter;
// use crate::ai::providers::create_provider_from_db;
// use crate::database::connection_manager::ConnectionManager;
use crate::database::DbPool;
use crate::skills::{
    get_or_init_registry, reload_registry, SelectionContext, SkillInfo,
    SkillSettings, SkillSettingsService, SkillUserConfig,
};

/// List all available skills.
#[tauri::command]
pub async fn list_available_skills_command(
    _pool: State<'_, DbPool>,
) -> Result<Vec<SkillInfo>, String> {
    let registry = get_or_init_registry().await;
    Ok(registry.get_skill_infos())
}

/// Get detailed information about a specific skill.
#[tauri::command]
pub async fn get_skill_details_command(
    skill_id: String,
    _pool: State<'_, DbPool>,
) -> Result<crate::skills::SkillDefinition, String> {
    let registry = get_or_init_registry().await;
    registry
        .get(&skill_id)
        .ok_or_else(|| format!("Skill not found: {}", skill_id))
}

/// Get skill settings for a workspace.
#[tauri::command]
pub async fn get_skill_settings_command(
    workspace_id: String,
    pool: State<'_, DbPool>,
) -> Result<SkillSettings, String> {
    let service = SkillSettingsService::new(pool.inner().clone());
    service
        .get_settings(&workspace_id)
        .map_err(|e| format!("Failed to get skill settings: {}", e))
}

/// Set whether a skill is enabled for a workspace.
#[tauri::command]
pub async fn set_skill_enabled_command(
    workspace_id: String,
    skill_id: String,
    enabled: bool,
    pool: State<'_, DbPool>,
) -> Result<(), String> {
    let service = SkillSettingsService::new(pool.inner().clone());
    service
        .set_skill_enabled(&workspace_id, &skill_id, enabled)
        .map_err(|e| format!("Failed to update skill setting: {}", e))
}

/// Update skill configuration for a workspace.
#[tauri::command]
pub async fn update_skill_config_command(
    workspace_id: String,
    skill_id: String,
    enabled: bool,
    priority_override: Option<i32>,
    pool: State<'_, DbPool>,
) -> Result<(), String> {
    let service = SkillSettingsService::new(pool.inner().clone());
    let config = SkillUserConfig {
        enabled,
        priority_override,
    };
    service
        .update_skill_config(&workspace_id, &skill_id, config)
        .map_err(|e| format!("Failed to update skill config: {}", e))
}

/// Initialize default skill settings for a workspace.
#[tauri::command]
pub async fn initialize_skill_defaults_command(
    workspace_id: String,
    pool: State<'_, DbPool>,
) -> Result<(), String> {
    let registry = get_or_init_registry().await;

    // Get skills that are enabled by default
    let default_enabled: Vec<String> = registry
        .list_all()
        .into_iter()
        .filter(|s| s.feature.default_enabled)
        .map(|s| s.id)
        .collect();

    let service = SkillSettingsService::new(pool.inner().clone());
    service
        .initialize_defaults(&workspace_id, &default_enabled)
        .map_err(|e| format!("Failed to initialize defaults: {}", e))
}

/*
// Database-specific skill execution commands - commented out for boilerplate
// Re-implement these if you need database-backed skill execution

/// Execute a request using the skill system.
#[tauri::command]
pub async fn execute_with_skills_command(
    workspace_id: String,
    request: String,
    entity_type: Option<String>,
    task_hint: Option<String>,
    provider_id: String,
    api_key: String,
    connection_manager: State<'_, Arc<ConnectionManager>>,
    pool: State<'_, DbPool>,
) -> Result<SkillOutput, String> {
    Err("Database-backed skill execution not implemented in boilerplate".to_string())
}

/// Execute a specific skill by ID.
#[tauri::command]
pub async fn execute_skill_command(
    workspace_id: String,
    skill_id: String,
    request: String,
    provider_id: String,
    api_key: String,
    connection_manager: State<'_, Arc<ConnectionManager>>,
    pool: State<'_, DbPool>,
) -> Result<SkillOutput, String> {
    Err("Database-backed skill execution not implemented in boilerplate".to_string())
}
*/

/// Reload skills from all sources.
#[tauri::command]
pub async fn reload_skills_command() -> Result<(), String> {
    reload_registry()
        .await
        .map_err(|e| format!("Failed to reload skills: {}", e))
}

/// List skills grouped by category.
#[tauri::command]
pub async fn list_skills_by_category_command(
) -> Result<std::collections::HashMap<String, Vec<SkillInfo>>, String> {
    let registry = get_or_init_registry().await;

    let all_skills = registry.get_skill_infos();
    let mut by_category: std::collections::HashMap<String, Vec<SkillInfo>> =
        std::collections::HashMap::new();

    for skill in all_skills {
        by_category
            .entry(skill.category.clone())
            .or_default()
            .push(skill);
    }

    Ok(by_category)
}

/// Toggle auto-select mode for a workspace.
#[tauri::command]
pub async fn set_skill_auto_select_command(
    workspace_id: String,
    auto_select: bool,
    pool: State<'_, DbPool>,
) -> Result<(), String> {
    let service = SkillSettingsService::new(pool.inner().clone());
    service
        .set_auto_select(&workspace_id, auto_select)
        .map_err(|e| format!("Failed to update auto-select: {}", e))
}

/// Skill suggestion for a given request.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSuggestion {
    pub skill_id: String,
    pub skill_name: String,
    pub description: String,
    pub relevance_score: f32,
    pub matched_triggers: Vec<String>,
}

/// Get skill suggestions for a user request.
///
/// Returns a list of skills that might be relevant for the given request,
/// based on trigger matching. This is useful for showing skill chips in the UI.
#[tauri::command]
pub async fn suggest_skills_command(
    workspace_id: String,
    request: String,
    database_type: Option<String>,
    pool: State<'_, DbPool>,
) -> Result<Vec<SkillSuggestion>, String> {
    use crate::skills::SkillSelector;

    let registry = get_or_init_registry().await;

    // Get enabled skills for this workspace
    let settings_service = SkillSettingsService::new(pool.inner().clone());
    let settings = settings_service
        .get_settings(&workspace_id)
        .map_err(|e| format!("Failed to get skill settings: {}", e))?;

    // Build selection context
    let context = SelectionContext {
        request: request.clone(),
        database_type,
        entity_type: None,
        task_hint: None,
        available_context: vec!["schema".to_string(), "database_type".to_string()],
        enabled_skills: settings.enabled_skills,
    };

    // Use selector to pre-filter matching skills
    let selector = SkillSelector::new(registry.clone());
    let candidates = selector.pre_filter(&context);

    // Convert to suggestions with relevance scoring
    let request_lower = request.to_lowercase();
    let suggestions: Vec<SkillSuggestion> = candidates
        .into_iter()
        .map(|skill| {
            // Calculate relevance based on trigger matches
            let mut matched_triggers = Vec::new();
            let mut score: f32 = 0.0;

            for trigger in &skill.triggers.task_types {
                if request_lower.contains(&trigger.to_lowercase()) {
                    matched_triggers.push(trigger.clone());
                    score += 0.3;
                }
            }

            for entity in &skill.triggers.entity_types {
                if request_lower.contains(&entity.to_lowercase()) {
                    matched_triggers.push(entity.clone());
                    score += 0.2;
                }
            }

            // Boost score if skill name appears in request
            if request_lower.contains(&skill.name.to_lowercase()) {
                score += 0.5;
            }

            // Cap at 1.0
            score = score.min(1.0);

            SkillSuggestion {
                skill_id: skill.id.clone(),
                skill_name: skill.name.clone(),
                description: skill.description.clone(),
                relevance_score: score,
                matched_triggers,
            }
        })
        .filter(|s| s.relevance_score > 0.0 || !s.matched_triggers.is_empty())
        .collect();

    // Sort by relevance score descending
    let mut sorted = suggestions;
    sorted.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(sorted)
}
