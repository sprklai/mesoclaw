use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::providers::{create_provider, ProviderType};
use crate::ai::types::{CompletionRequest, Message};
use crate::database::models::settings::SettingsUpdate;
use crate::database::DbPool;
use crate::services::settings::update_settings;

/// LLM provider configuration (for testing only - API keys are stored in Stronghold)
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LLMProviderConfig {
    pub provider_id: String,
    pub model_id: String,
    pub api_key: String,
}

/// Test result for LLM provider
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    pub model: Option<String>,
}

/// Configure the LLM provider (save provider and model preference to database)
/// API keys are now stored in Stronghold on the frontend
///
/// If workspace_id is provided, stores the configuration for that specific workspace.
/// Otherwise, stores the global default configuration.
#[tauri::command]
pub fn configure_llm_provider_command(
    pool: State<'_, DbPool>,
    provider_id: String,
    model_id: String,
    workspace_id: Option<String>,
) -> Result<(), String> {
    if let Some(_ws_id) = workspace_id {
        // Workspace-specific configuration removed - database-specific
        return Err("Workspace-specific LLM config not implemented in boilerplate".to_string());
    } else {
        // Store global configuration
        let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;
        let settings_update = SettingsUpdate {
            llm_model: Some(format!("{}/{}", provider_id, model_id)),
            ..Default::default()
        };
        update_settings(&mut conn, settings_update)
            .map_err(|e| format!("Failed to save provider/model preference: {}", e))?;
    }

    Ok(())
}

/// Get the current LLM provider configuration
/// Returns both provider_id and model_id
///
/// If workspace_id is provided, returns workspace-specific config (or None if not set).
/// Otherwise, returns global default configuration.
#[tauri::command]
pub fn get_llm_provider_config_command(
    pool: State<'_, DbPool>,
    workspace_id: Option<String>,
) -> Result<LLMProviderConfigResponse, String> {
    if workspace_id.is_some() {
        // Workspace-specific configuration removed - database-specific
        return Err("Workspace-specific LLM config not implemented in boilerplate".to_string());
    }

    // Get global configuration only
    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;
    let settings = crate::services::settings::get_settings(&mut conn)
        .map_err(|e| format!("Failed to load settings: {}", e))?;

    let parts: Vec<&str> = settings.llm_model.split('/').collect();
    let (provider_id, model_id) = if parts.len() >= 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        ("vercel-ai-gateway".to_string(), settings.llm_model.clone())
    };

    Ok(LLMProviderConfigResponse {
        provider_id,
        model_id,
    })
}

/// Response for get_llm_provider_config_command
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LLMProviderConfigResponse {
    pub provider_id: String,
    pub model_id: String,
}

/// Test the LLM provider connection
#[tauri::command]
pub async fn test_llm_provider_command(config: LLMProviderConfig) -> Result<TestResult, String> {
    // Parse provider type
    let provider_type = ProviderType::from_id(&config.provider_id)
        .ok_or_else(|| format!("Unknown provider: {}", config.provider_id))?;

    // Create a temporary provider instance
    let provider = create_provider(
        provider_type,
        &config.api_key,
        None, // Use default base_url from provider config
        Some(&config.model_id),
    )
    .map_err(|e| format!("Failed to create provider: {}", e))?;

    // Make a simple test request
    let request = CompletionRequest::new(
        &config.model_id,
        vec![Message::user("Say 'test successful' if you can read this.")],
    )
    .with_max_tokens(20);

    match provider.complete(request).await {
        Ok(response) => Ok(TestResult {
            success: true,
            message: format!("Connection successful! Model: {}", response.model),
            model: Some(response.model),
        }),
        Err(msg) if msg.contains("401") || msg.contains("403") => {
            Ok(TestResult {
                success: false,
                message: "Invalid API key. Please check your credentials.".to_string(),
                model: None,
            })
        }
        Err(msg) if msg.contains("404") => Ok(TestResult {
            success: false,
            message: format!("Model '{}' not found or not accessible.", config.model_id),
            model: None,
        }),
        Err(e) => Ok(TestResult {
            success: false,
            message: format!("Connection failed: {}", e),
            model: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_config_serialization() {
        let config = LLMProviderConfig {
            provider_id: "vercel-ai-gateway".to_string(),
            model_id: "anthropic/claude-sonnet-4.5".to_string(),
            api_key: "test-key".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LLMProviderConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.provider_id, "vercel-ai-gateway");
        assert_eq!(deserialized.model_id, "anthropic/claude-sonnet-4.5");
        assert_eq!(deserialized.api_key, "test-key");
    }

    #[test]
    fn test_llm_provider_config_response_serialization() {
        let response = LLMProviderConfigResponse {
            provider_id: "openrouter".to_string(),
            model_id: "anthropic/claude-3.5-sonnet".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: LLMProviderConfigResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.provider_id, "openrouter");
        assert_eq!(deserialized.model_id, "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_test_result_serialization() {
        let result = TestResult {
            success: true,
            message: "Test successful".to_string(),
            model: Some("gpt-4".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TestResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.success, true);
        assert_eq!(deserialized.message, "Test successful");
        assert_eq!(deserialized.model, Some("gpt-4".to_string()));
    }
}
