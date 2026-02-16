use diesel::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::database::DbPool;
use crate::database::models::ai_provider::{AIProvider, NewAIModel};
use crate::database::schema::{ai_models, ai_providers};

/// Ollama model response from GET /api/tags
#[derive(Debug, Serialize, Deserialize)]
struct OllamaModel {
    name: String,
    modified_at: String,
    size: Option<i64>,
}

/// Ollama models response
#[derive(Debug, Serialize, Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

/// Discover and sync Ollama models from the local Ollama instance
///
/// This makes a GET request to http://localhost:11434/api/tags
/// to discover what models the user has installed locally.
#[tauri::command]
pub async fn discover_ollama_models_command(
    pool: State<'_, DbPool>,
) -> Result<usize, String> {
    println!("[Ollama Discovery] Starting model discovery...");

    let mut conn = pool
        .get()
        .map_err(|e| {
            println!("[Ollama Discovery] Database connection failed: {}", e);
            format!("Database error: {}", e)
        })?;

    // Check if Ollama provider exists
    let provider = ai_providers::table
        .filter(ai_providers::id.eq("ollama"))
        .filter(ai_providers::is_active.eq(1))
        .first::<AIProvider>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to query provider: {}", e))?;

    if provider.is_none() {
        println!("[Ollama Discovery] Ollama provider not found or inactive");
        return Err("Ollama provider not found. Please ensure the provider is active.".to_string());
    }

    let provider = provider.unwrap();
    let base_url = &provider.base_url;
    println!("[Ollama Discovery] Using base_url: {}", base_url);

    // Parse base URL to get the host for Ollama's native API
    // Format: http://localhost:11434/v1 (OpenAI-compatible) or http://localhost:11434 (native)
    // We need to use the native Ollama API endpoint: /api/tags
    let api_base = base_url.trim_end_matches("/v1");

    println!("[Ollama Discovery] Connecting to Ollama at: {}", api_base);

    // Make request to Ollama's native API
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| {
            println!("[Ollama Discovery] Failed to create HTTP client: {}", e);
            format!("Failed to create HTTP client: {}", e)
        })?;

    let url = format!("{}/api/tags", api_base);
    println!("[Ollama Discovery] Requesting URL: {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            println!("[Ollama Discovery] HTTP request failed: {}", e);
            format!("Failed to connect to Ollama: {}. Make sure Ollama is running at {}", e, api_base)
        })?;

    let status = response.status();
    println!("[Ollama Discovery] Response status: {}", status);

    if !status.is_success() {
        return Err(format!(
            "Ollama returned error: {}. Make sure Ollama is running at {}",
            status,
            api_base
        ));
    }

    let ollama_response: OllamaModelsResponse = response
        .json()
        .await
        .map_err(|e| {
            println!("[Ollama Discovery] Failed to parse JSON: {}", e);
            format!("Failed to parse Ollama response: {}", e)
        })?;

    println!("[Ollama Discovery] Found {} model(s)", ollama_response.models.len());
    for (i, model) in ollama_response.models.iter().enumerate() {
        println!("  - Model {}: {}", i + 1, model.name);
    }

    // Sync models to database
    let mut added_count = 0;
    let mut skipped_count = 0;
    let total_models = ollama_response.models.len();

    println!("[Ollama Discovery] Syncing {} model(s) to database...", total_models);

    for ollama_model in ollama_response.models {
        let model_id = &ollama_model.name;

        // Create a safe ID by replacing both slashes and colons (common in Ollama model names)
        let safe_id = model_id.replace('/', "-").replace(':', "-");
        let db_id = format!("ollama_{}", safe_id);

        // Check if model already exists using the unique constraint (provider_id, model_id)
        let existing = ai_models::table
            .filter(ai_models::provider_id.eq("ollama"))
            .filter(ai_models::model_id.eq(model_id))
            .select(ai_models::id)
            .first::<String>(&mut conn)
            .optional()
            .map_err(|e| {
                println!("[Ollama Discovery] Database query failed for model {}: {}", model_id, e);
                format!("Failed to check existing model: {}", e)
            })?;

        if existing.is_some() {
            println!("[Ollama Discovery] Model {} already exists (provider_id=ollama, model_id={}), skipping", db_id, model_id);
            skipped_count += 1;
            continue; // Skip if already exists
        }

        // Insert new model
        println!("[Ollama Discovery] Adding new model: {}", db_id);

        // Convert i64 size to i32 context limit (cap at i32::MAX if needed)
        let context_limit = ollama_model.size.map(|s| {
            if s > i32::MAX as i64 {
                i32::MAX
            } else {
                s as i32
            }
        });

        let new_model = NewAIModel::new(
            db_id.clone(),
            "ollama",
            model_id,
            &ollama_model.name,
            context_limit,
            false,
        );

        diesel::insert_into(ai_models::table)
            .values(&new_model)
            .execute(&mut conn)
            .map_err(|e| {
                println!("[Ollama Discovery] Failed to insert model {}: {}", db_id, e);
                format!("Failed to insert model: {}", e)
            })?;

        println!("[Ollama Discovery] Successfully inserted model: {}", db_id);
        added_count += 1;
    }

    // Return meaningful message
    if total_models == 0 {
        println!("[Ollama Discovery] No models found in Ollama");
        return Err("No models found in Ollama. Make sure you have pulled at least one model with 'ollama pull <model-name>'".to_string());
    }

    if added_count == 0 && skipped_count > 0 {
        println!("[Ollama Discovery] All {} model(s) already exist in database", skipped_count);
        println!("[Ollama Discovery] Discovery complete - no new models");
        // Return 0 - frontend will show appropriate message
        return Ok(0);
    }

    println!("[Ollama Discovery] Added {} new model(s)", added_count);
    println!("[Ollama Discovery] Discovery complete - success");
    Ok(added_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_models_response_parsing() {
        let json = r#"{"models":[{"name":"llama3","modified_at":"2024-01-01T00:00:00Z","size":4000000000}]}"#;

        let response: OllamaModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.models.len(), 1);
        assert_eq!(response.models[0].name, "llama3");
    }
}
