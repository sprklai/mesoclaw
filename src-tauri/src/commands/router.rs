//! Router Tauri Commands
//!
//! Exposes router functionality to the frontend.

use std::sync::Arc;
use tauri::State;

use crate::ai::discovery::ProviderConfig;
use crate::ai::providers::router::{ModelModality, RoutingProfile, TaskType};
use crate::database::models::{DiscoveredModelData, RouterConfigData};
use crate::services::{model_registry::ModelRegistry, router::RouterService};

/// Router state for Tauri commands
#[derive(Clone)]
pub struct RouterState {
    pub router: Arc<RouterService>,
    pub registry: Arc<ModelRegistry>,
}

/// Get the current router configuration
#[tauri::command]
pub async fn get_router_config(state: State<'_, RouterState>) -> Result<RouterConfigData, String> {
    Ok(state.router.get_config().await)
}

/// Set the active routing profile
#[tauri::command]
pub async fn set_router_profile(
    profile: String,
    state: State<'_, RouterState>,
) -> Result<(), String> {
    let profile = match profile.as_str() {
        "eco" => RoutingProfile::Eco,
        "premium" => RoutingProfile::Premium,
        _ => RoutingProfile::Balanced,
    };

    state.router.set_profile(profile).await
}

/// Get all discovered models
#[tauri::command]
pub async fn get_discovered_models(
    state: State<'_, RouterState>,
) -> Result<Vec<DiscoveredModelData>, String> {
    Ok(state.registry.get_all_models().await)
}

/// Get discovered models for a specific provider
#[tauri::command]
pub async fn get_discovered_models_by_provider(
    provider_id: String,
    state: State<'_, RouterState>,
) -> Result<Vec<DiscoveredModelData>, String> {
    Ok(state.registry.get_models_by_provider(&provider_id).await)
}

/// Discover models from a provider
#[tauri::command]
pub async fn discover_models(
    provider_id: String,
    base_url: String,
    api_key: Option<String>,
    state: State<'_, RouterState>,
) -> Result<usize, String> {
    let config = ProviderConfig::new(&provider_id, &base_url).with_timeout(30);

    let config = if let Some(key) = api_key {
        config.with_api_key(key)
    } else {
        config
    };

    let count = state
        .registry
        .discover_and_sync(&provider_id, &config)
        .await?;

    // Record discovery timestamp
    state.router.record_discovery().await?;

    Ok(count)
}

/// Set a task override
#[tauri::command]
pub async fn set_task_override(
    task: String,
    model_id: String,
    state: State<'_, RouterState>,
) -> Result<(), String> {
    let task_type = match task.as_str() {
        "code" => TaskType::Code,
        "general" => TaskType::General,
        "fast" => TaskType::Fast,
        "creative" => TaskType::Creative,
        "analysis" => TaskType::Analysis,
        _ => TaskType::Other,
    };

    state.router.set_task_override(task_type, model_id).await
}

/// Clear a task override
#[tauri::command]
pub async fn clear_task_override(
    task: String,
    state: State<'_, RouterState>,
) -> Result<(), String> {
    let task_type = match task.as_str() {
        "code" => TaskType::Code,
        "general" => TaskType::General,
        "fast" => TaskType::Fast,
        "creative" => TaskType::Creative,
        "analysis" => TaskType::Analysis,
        _ => TaskType::Other,
    };

    state.router.clear_task_override(task_type).await
}

/// Route a message to get the best model
#[tauri::command]
pub async fn route_message(
    message: String,
    state: State<'_, RouterState>,
) -> Result<Option<String>, String> {
    Ok(state.router.route(&message).await)
}

/// Route a message with modality requirements
#[tauri::command]
pub async fn route_message_with_modalities(
    message: String,
    modalities: Vec<String>,
    state: State<'_, RouterState>,
) -> Result<Option<String>, String> {
    let modality_types: Vec<ModelModality> = modalities
        .into_iter()
        .filter_map(|m| match m.as_str() {
            "text" => Some(ModelModality::Text),
            "image" => Some(ModelModality::Image),
            "image_generation" => Some(ModelModality::ImageGeneration),
            "audio_transcription" => Some(ModelModality::AudioTranscription),
            "audio_generation" => Some(ModelModality::AudioGeneration),
            "video" => Some(ModelModality::Video),
            "embedding" => Some(ModelModality::Embedding),
            _ => None,
        })
        .collect();

    Ok(state
        .router
        .route_with_modalities(&message, &modality_types)
        .await)
}

/// Get available models for current profile
#[tauri::command]
pub async fn get_available_models(state: State<'_, RouterState>) -> Result<Vec<String>, String> {
    Ok(state.router.get_available_models().await)
}

/// Check if a provider is available
#[tauri::command]
pub async fn is_provider_available(
    provider_id: String,
    base_url: String,
    api_key: Option<String>,
    state: State<'_, RouterState>,
) -> Result<bool, String> {
    let config = ProviderConfig::new(&provider_id, &base_url).with_timeout(5);

    let config = if let Some(key) = api_key {
        config.with_api_key(key)
    } else {
        config
    };

    Ok(state
        .registry
        .is_provider_available(&provider_id, &config)
        .await)
}

/// Reload models from database
#[tauri::command]
pub async fn reload_models(state: State<'_, RouterState>) -> Result<usize, String> {
    state.registry.load_from_database().await
}

/// Get model count
#[tauri::command]
pub async fn get_model_count(state: State<'_, RouterState>) -> Result<usize, String> {
    Ok(state.registry.model_count().await)
}

/// Initialize router state from database
#[tauri::command]
pub async fn initialize_router(state: State<'_, RouterState>) -> Result<(), String> {
    state.router.load_from_database().await?;
    state.registry.load_from_database().await?;
    log::info!("[Router] Initialized router and registry from database");
    Ok(())
}
