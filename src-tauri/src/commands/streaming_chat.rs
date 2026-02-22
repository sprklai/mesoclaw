use diesel::prelude::*;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::ai::provider::LLMProvider;
use crate::ai::providers::OpenAICompatibleProvider;
use crate::ai::types::{CompletionRequest, Message};
use crate::commands::ai_providers::create_test_config;
use crate::database::DbPool;
use crate::database::models::ai_provider::AIProvider;
use crate::database::schema::ai_providers;

/// Chat message request
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageRequest {
    pub provider_id: String,
    pub model_id: String,
    pub api_key: String,
    pub messages: Vec<ChatMessage>,
    pub session_id: String, // Unique ID for this chat session to target events
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

/// Stream chat completion with SSE-style events
#[tauri::command]
#[tracing::instrument(name = "command.stream_chat", skip_all, fields(provider = %request.provider_id, model = %request.model_id, session = %request.session_id))]
pub async fn stream_chat_command(
    app: AppHandle,
    pool: State<'_, DbPool>,
    request: ChatMessageRequest,
) -> Result<(), String> {
    // Get database connection
    let mut conn = pool
        .get()
        .map_err(|e| format!("Database connection failed: {}", e))?;

    // Load provider from database
    let provider_record = ai_providers::table
        .filter(ai_providers::id.eq(&request.provider_id))
        .first::<AIProvider>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to load provider: {}", e))?
        .ok_or_else(|| format!("Provider not found: {}", request.provider_id))?;

    // For providers that don't require API keys (like Ollama), use empty string
    // The OpenAI-compatible provider will handle this correctly by not adding auth header
    let api_key = if provider_record.requires_api_key == 0 {
        String::new()
    } else {
        request.api_key.clone()
    };

    // Create provider config with provider-specific headers
    let config = create_test_config(
        &provider_record.id,
        &api_key,
        &provider_record.base_url,
        &request.model_id,
    );

    let provider = std::sync::Arc::new(
        OpenAICompatibleProvider::new(config, &request.provider_id)
            .map_err(|e| format!("Failed to create provider: {}", e))?,
    );

    // Convert ChatMessage to AI Message
    let messages: Vec<Message> = request
        .messages
        .iter()
        .map(|m| {
            if m.role == "user" {
                Message::user(&m.content)
            } else {
                Message::assistant(&m.content)
            }
        })
        .collect();

    // Create completion request with minimal parameters for maximum compatibility
    // Some newer OpenAI models (o1-preview, o1-mini, etc.) reject custom temperature
    // and other parameters, so we keep the request minimal and let providers use defaults
    let mut completion_request = CompletionRequest::new(&request.model_id, messages);

    // Only add max_tokens for providers that are known to support it
    // Some OpenAI reasoning models don't accept this parameter
    if provider_record.id != "openai" || !request.model_id.starts_with("o1") {
        completion_request = completion_request.with_max_tokens(2000);
    }

    // Start streaming
    let session_id = request.session_id.clone();

    // Emit start event
    app.emit(
        &format!("chat-stream-{}", session_id),
        serde_json::json!({
            "type": "start"
        }),
    )
    .map_err(|e| format!("Failed to emit start event: {}", e))?;

    // Start streaming
    let mut stream = provider
        .stream(completion_request)
        .await
        .map_err(|e| format!("Failed to start stream: {}", e))?;

    // Accumulate the full content
    let mut full_content = String::new();

    // Process stream chunks
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Add delta to accumulated content
                full_content.push_str(&chunk.delta);

                // Emit token event with accumulated content
                app.emit(
                    &format!("chat-stream-{}", session_id),
                    serde_json::json!({
                        "type": "token",
                        "content": full_content
                    }),
                )
                .map_err(|e| format!("Failed to emit token event: {}", e))?;

                // Check if this is the final chunk
                if chunk.is_final {
                    break;
                }
            }
            Err(e) => {
                // Emit error event
                app.emit(
                    &format!("chat-stream-{}", session_id),
                    serde_json::json!({
                        "type": "error",
                        "error": e.to_string()
                    }),
                )
                .map_err(|err| format!("Failed to emit error event: {}", err))?;

                return Err(format!("Stream error: {}", e));
            }
        }
    }

    // Emit done event
    app.emit(
        &format!("chat-stream-{}", session_id),
        serde_json::json!({
            "type": "done"
        }),
    )
    .map_err(|e| format!("Failed to emit done event: {}", e))?;

    Ok(())
}
