use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::provider::LLMProvider;
use crate::ai::providers::openai_compatible::{OpenAICompatibleConfig, OpenAICompatibleProvider};
use crate::ai::types::{CompletionRequest, Message};
use crate::database::models::ai_provider::{
    AIModel, AIModelData, AIProvider, AIProviderData, NewAIModel, NewAIProvider, ProviderWithModels,
};
use crate::database::schema::settings;
use crate::database::DbPool;
use crate::database::schema::{ai_models, ai_providers};

/// Service name for keychain storage (consistent across the app)
const KEYCHAIN_SERVICE: &str = "com.sprklai.aiboilerplate";

/// Provider with API key status response
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWithKeyStatusResponse {
    #[serde(flatten)]
    pub provider: AIProviderData,
    pub has_api_key: bool,
}

/// Test result for provider connection
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
    pub model: Option<String>,
}

/// List all providers with their associated models
///
/// This returns all active providers from the database along with
/// their associated active models.
#[tauri::command]
pub fn list_ai_providers_command(
    pool: State<'_, DbPool>,
) -> Result<Vec<ProviderWithModels>, String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Get all active providers
    let providers = ai_providers::table
        .filter(ai_providers::is_active.eq(1))
        .load::<AIProvider>(&mut conn)
        .map_err(|e| format!("Failed to load providers: {}", e))?;

    let mut result = Vec::new();

    // For each provider, get its active models
    for provider in providers {
        let models = ai_models::table
            .filter(ai_models::provider_id.eq(&provider.id))
            .filter(ai_models::is_active.eq(1))
            .load::<AIModel>(&mut conn)
            .map_err(|e| format!("Failed to load models: {}", e))?;

        let provider_data = AIProviderData::from(provider.clone());
        let models_data: Vec<AIModelData> = models.into_iter().map(AIModelData::from).collect();

        result.push(ProviderWithModels::new(provider_data, models_data));
    }

    Ok(result)
}

/// List all providers with their API key status
///
/// This returns all active providers and indicates whether they have
/// an API key stored in the keychain. Useful for the settings UI.
#[tauri::command]
pub async fn list_providers_with_key_status_command(
    pool: State<'_, DbPool>,
) -> Result<Vec<ProviderWithKeyStatusResponse>, String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Get all active providers
    let providers = ai_providers::table
        .filter(ai_providers::is_active.eq(1))
        .load::<AIProvider>(&mut conn)
        .map_err(|e| format!("Failed to load providers: {}", e))?;

    let mut result = Vec::new();

    // Check API key status for each provider
    for provider in providers {
        // For providers that don't require API keys (like Ollama), has_api_key is always true
        let has_api_key = if provider.requires_api_key == 0 {
            true // Local providers don't need API keys
        } else {
            // Check if key exists in keychain
            check_api_key_exists(&provider.id).await.unwrap_or(false)
        };

        let provider_data = AIProviderData::from(provider);

        result.push(ProviderWithKeyStatusResponse {
            provider: provider_data,
            has_api_key,
        });
    }

    Ok(result)
}

/// Get a specific provider by ID
///
/// Returns the provider details along with its models.
#[tauri::command]
pub fn get_provider_by_id_command(
    pool: State<'_, DbPool>,
    provider_id: String,
) -> Result<ProviderWithModels, String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Get the provider
    let provider = ai_providers::table
        .filter(ai_providers::id.eq(&provider_id))
        .first::<AIProvider>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to load provider: {}", e))?
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    // Get the provider's active models
    let models = ai_models::table
        .filter(ai_models::provider_id.eq(&provider_id))
        .filter(ai_models::is_active.eq(1))
        .load::<AIModel>(&mut conn)
        .map_err(|e| format!("Failed to load models: {}", e))?;

    let provider_data = AIProviderData::from(provider);
    let models_data: Vec<AIModelData> = models.into_iter().map(AIModelData::from).collect();

    Ok(ProviderWithModels::new(provider_data, models_data))
}

/// Test a provider's connection
///
/// Tests the provider connection by making a simple completion request.
/// Returns success status, latency, and the model that responded.
#[tauri::command]
pub async fn test_provider_connection_command(
    pool: State<'_, DbPool>,
    provider_id: String,
    api_key: String,
) -> Result<ProviderTestResult, String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Get the provider from database (to verify it exists)
    let provider = ai_providers::table
        .filter(ai_providers::id.eq(&provider_id))
        .first::<AIProvider>(&mut conn)
        .map_err(|e| format!("Failed to load provider: {}", e))?;

    // For local providers that don't require API keys, use a placeholder
    // The provider implementation will ignore this for local services like Ollama
    let effective_api_key = if provider.requires_api_key == 0 {
        "local-provider-no-key-needed"
    } else {
        &api_key
    };

    // Get the first available model for this provider
    let model = ai_models::table
        .filter(ai_models::provider_id.eq(&provider_id))
        .filter(ai_models::is_active.eq(1))
        .first::<AIModel>(&mut conn)
        .map_err(|e| format!("Failed to load model: {}", e))?;

    // Create the provider instance specifically for testing with the test model
    // We don't use create_provider_from_db here because that loads the default model from settings
    // Instead, we create the provider directly with the test model we want to use
    let config = OpenAICompatibleConfig::with_model(
        effective_api_key,
        &provider.base_url,
        &model.model_id,
    );

    let provider_instance = OpenAICompatibleProvider::new(config, &provider.id)
        .map_err(|e| format!("Failed to create provider: {}", e))?;
    let provider_instance = std::sync::Arc::new(provider_instance);

    // Record start time
    let start = std::time::Instant::now();

    // Make a simple test request
    let request = CompletionRequest::new(
        &model.model_id,
        vec![Message::user("Say 'test successful' if you can read this.")],
    )
    .with_max_tokens(20);

    match provider_instance.complete(request).await {
        Ok(response) => {
            let latency = start.elapsed().as_millis() as u64;
            Ok(ProviderTestResult {
                success: true,
                latency_ms: Some(latency),
                message: format!("Connection successful! Model: {}", response.model),
                model: Some(response.model),
            })
        }
        Err(msg) if msg.contains("401") || msg.contains("403") => {
            Ok(ProviderTestResult {
                success: false,
                latency_ms: None,
                message: "Invalid API key. Please check your credentials.".to_string(),
                model: None,
            })
        }
        Err(msg) if msg.contains("404") => Ok(ProviderTestResult {
            success: false,
            latency_ms: None,
            message: format!("Model '{}' not found or not accessible.", model.model_id),
            model: None,
        }),
        Err(e) => Ok(ProviderTestResult {
            success: false,
            latency_ms: None,
            message: format!("Connection failed: {}", e),
            model: None,
        }),
    }
}

/// Add a custom model to the database
///
/// Allows users to add their own models for a provider.
/// Custom models are marked with is_custom=1 in the database.
#[tauri::command]
pub fn add_custom_model_command(
    pool: State<'_, DbPool>,
    provider_id: String,
    model_id: String,
    display_name: String,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Verify the provider exists
    let provider_exists = ai_providers::table
        .filter(ai_providers::id.eq(&provider_id))
        .first::<AIProvider>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to check provider: {}", e))?
        .is_some();

    if !provider_exists {
        return Err(format!("Provider not found: {}", provider_id));
    }

    // Check if model already exists for this provider
    let existing = ai_models::table
        .filter(ai_models::provider_id.eq(&provider_id))
        .filter(ai_models::model_id.eq(&model_id))
        .first::<AIModel>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to check existing model: {}", e))?;

    if existing.is_some() {
        return Err(format!("Model already exists for this provider: {}", model_id));
    }

    // Generate a unique ID for the custom model
    let custom_model_id = format!("custom-{}-{}", provider_id, model_id.replace('/', "-"));

    // Create and insert the new custom model
    let new_model = crate::database::models::ai_provider::NewAIModel::custom(
        &custom_model_id,
        &provider_id,
        &model_id,
        &display_name,
    );

    diesel::insert_into(ai_models::table)
        .values(&new_model)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to add custom model: {}", e))?;

    Ok(())
}

/// Delete a model from the database
///
/// Only custom models (is_custom=1) can be deleted.
/// Built-in models cannot be deleted.
#[tauri::command]
pub fn delete_model_command(
    pool: State<'_, DbPool>,
    model_id: String,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Find the model
    let model = ai_models::table
        .filter(ai_models::id.eq(&model_id))
        .first::<AIModel>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to find model: {}", e))?;

    match model {
        Some(m) => {
            // Only custom models can be deleted
            if m.is_custom == 0 {
                return Err("Cannot delete built-in models".to_string());
            }

            // Delete the model
            diesel::delete(ai_models::table.filter(ai_models::id.eq(&model_id)))
                .execute(&mut conn)
                .map_err(|e| format!("Failed to delete model: {}", e))?;

            Ok(())
        }
        None => Err("Model not found".to_string()),
    }
}

/// Reactivate a provider and its models
///
/// This command reactivates a provider that was previously deactivated.
/// Useful for restoring providers like Anthropic that were disabled.
#[tauri::command]
pub fn reactivate_provider_command(
    pool: State<'_, DbPool>,
    provider_id: String,
) -> Result<String, String> {
    let mut conn = pool
        .get()
        .map_err(|e| format!("Database error: {}", e))?;

    // Reactivate the provider
    let updated = diesel::update(ai_providers::table.filter(ai_providers::id.eq(&provider_id)))
        .set(ai_providers::is_active.eq(1))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to reactivate provider: {}", e))?;

    if updated == 0 {
        return Err(format!("Provider not found: {}", provider_id));
    }

    // Reactivate all models for this provider
    let model_count = diesel::update(ai_models::table.filter(ai_models::provider_id.eq(&provider_id)))
        .set(ai_models::is_active.eq(1))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to reactivate models: {}", e))?;

    Ok(format!(
        "Reactivated provider '{}' and {} model(s)",
        provider_id, model_count
    ))
}

/// Update provider details
///
/// Allows updating the base URL and other settings for a provider.
/// Useful for configuring custom AI gateway URLs.
#[tauri::command]
pub fn update_provider_command(
    pool: State<'_, DbPool>,
    provider_id: String,
    base_url: String,
) -> Result<(), String> {
    let mut conn = pool
        .get()
        .map_err(|e| format!("Database error: {}", e))?;

    // Update the provider
    let updated = diesel::update(ai_providers::table.filter(ai_providers::id.eq(&provider_id)))
        .set(ai_providers::base_url.eq(&base_url))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update provider: {}", e))?;

    if updated == 0 {
        return Err(format!("Provider not found: {}", provider_id));
    }

    Ok(())
}

/// Check if an API key exists in the keychain for a provider
///
/// Note: We use the frontend KeychainStorage key format: api_key:provider
/// This must match the format used in src/lib/keychain-storage.ts
async fn check_api_key_exists(provider_id: &str) -> Result<bool, String> {
    // Use the same key format as frontend KeychainStorage
    let key = format!("api_key:{}", provider_id);

    // Create a keyring entry with the consistent service name
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &key)
        .map_err(|e| format!("Failed to create keychain entry: {}", e))?;

    // Try to get the password to check existence
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(format!("Failed to check keychain: {}", e)),
    }
}

/// Initial model specification for creating user-defined providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialModelSpec {
    pub model_id: String,
    pub display_name: Option<String>,
}

/// Global default model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalDefaultModel {
    pub provider_id: String,
    pub model_id: String,
}

/// Add a user-defined AI provider
///
/// Creates a new user-defined provider with initial models.
/// The provider is marked as user-defined and can be deleted by the user.
#[tauri::command]
pub fn add_user_provider_command(
    pool: State<'_, DbPool>,
    id: String,
    name: String,
    base_url: String,
    requires_api_key: bool,
    initial_models: Vec<InitialModelSpec>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Validate provider ID format (lowercase, alphanumeric with hyphens)
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err("Provider ID must be lowercase alphanumeric with hyphens only".to_string());
    }
    if id.len() < 3 || id.len() > 50 {
        return Err("Provider ID must be 3-50 characters".to_string());
    }

    // Check if provider already exists
    let existing = ai_providers::table
        .filter(ai_providers::id.eq(&id))
        .first::<AIProvider>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to check existing provider: {}", e))?;

    if existing.is_some() {
        return Err(format!("Provider with ID '{}' already exists", id));
    }

    // Require at least one model
    if initial_models.is_empty() {
        return Err("At least one initial model is required".to_string());
    }

    // Create the user-defined provider
    let new_provider = NewAIProvider::user_defined(&id, &name, &base_url, requires_api_key);

    diesel::insert_into(ai_providers::table)
        .values(&new_provider)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to create provider: {}", e))?;

    // Add initial models
    for model_spec in initial_models {
        let display_name = model_spec.display_name.unwrap_or_else(|| model_spec.model_id.clone());
        let model_db_id = format!("{}-{}", id, model_spec.model_id.replace('/', "-"));

        let new_model = NewAIModel::custom(&model_db_id, &id, &model_spec.model_id, &display_name);

        diesel::insert_into(ai_models::table)
            .values(&new_model)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to add model '{}': {}", model_spec.model_id, e))?;
    }

    Ok(())
}

/// Delete a user-defined AI provider
///
/// Deletes the provider, all associated models, and removes the API key from keychain.
/// Only user-defined providers can be deleted.
#[tauri::command]
pub async fn delete_user_provider_command(
    pool: State<'_, DbPool>,
    provider_id: String,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Check if provider exists and is user-defined
    let provider = ai_providers::table
        .filter(ai_providers::id.eq(&provider_id))
        .first::<AIProvider>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to check provider: {}", e))?
        .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

    if provider.is_user_defined == 0 {
        return Err("Cannot delete built-in providers. Only user-defined providers can be deleted.".to_string());
    }

    // Delete all models for this provider
    diesel::delete(ai_models::table.filter(ai_models::provider_id.eq(&provider_id)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete provider models: {}", e))?;

    // Delete the provider
    diesel::delete(ai_providers::table.filter(ai_providers::id.eq(&provider_id)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete provider: {}", e))?;

    // Delete API key from keychain (best effort)
    let key = format!("api_key:{}", provider_id);
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, &key) {
        let _ = entry.delete_password(); // Ignore errors - key might not exist
    }

    Ok(())
}

/// Get the global default model configuration
///
/// Returns the provider_id and model_id stored in settings,
/// or null if no default is configured.
#[tauri::command]
pub fn get_global_default_model_command(
    pool: State<'_, DbPool>,
) -> Result<Option<GlobalDefaultModel>, String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    let result: (Option<String>, Option<String>) = settings::table
        .select((settings::default_provider_id, settings::default_model_id))
        .first(&mut conn)
        .map_err(|e| format!("Failed to load settings: {}", e))?;

    match (result.0, result.1) {
        (Some(provider_id), Some(model_id)) => Ok(Some(GlobalDefaultModel {
            provider_id,
            model_id,
        })),
        _ => Ok(None),
    }
}

/// Set the global default model configuration
///
/// Updates the settings table with the selected provider and model.
#[tauri::command]
pub fn set_global_default_model_command(
    pool: State<'_, DbPool>,
    provider_id: String,
    model_id: String,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    // Verify provider exists
    let provider_exists = ai_providers::table
        .filter(ai_providers::id.eq(&provider_id))
        .first::<AIProvider>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to check provider: {}", e))?
        .is_some();

    if !provider_exists {
        return Err(format!("Provider not found: {}", provider_id));
    }

    // Verify model exists for this provider
    let model_exists = ai_models::table
        .filter(ai_models::provider_id.eq(&provider_id))
        .filter(ai_models::model_id.eq(&model_id))
        .first::<AIModel>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to check model: {}", e))?
        .is_some();

    if !model_exists {
        return Err(format!("Model '{}' not found for provider '{}'", model_id, provider_id));
    }

    // Update settings
    diesel::update(settings::table)
        .set((
            settings::default_provider_id.eq(Some(&provider_id)),
            settings::default_model_id.eq(Some(&model_id)),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update default model: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_test_result_serialization() {
        let result = ProviderTestResult {
            success: true,
            latency_ms: Some(250),
            message: "Connection successful".to_string(),
            model: Some("gpt-4".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"latencyMs\":250"));

        let deserialized: ProviderTestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.success, true);
        assert_eq!(deserialized.latency_ms, Some(250));
    }

    #[test]
    fn test_provider_with_key_status_response_serialization() {
        let provider_data = AIProviderData {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            requires_api_key: true,
            is_active: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_user_defined: false,
        };

        let response = ProviderWithKeyStatusResponse {
            provider: provider_data,
            has_api_key: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ProviderWithKeyStatusResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.provider.id, "openai");
        assert_eq!(deserialized.has_api_key, true);
    }
}
