use log::{debug, info};
use serde::{Deserialize, Serialize};

/// Available LLM models for chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    pub id: String,
    pub name: String,
    pub provider: String,
}

/// Get available LLM models
#[tauri::command]
pub fn get_available_models_command() -> Vec<AvailableModel> {
    info!("[chat] fetching available models");
    let result = vec![
        AvailableModel {
            id: "openai/gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider: "OpenAI".to_string(),
        },
        AvailableModel {
            id: "openai/gpt-4".to_string(),
            name: "GPT-4".to_string(),
            provider: "OpenAI".to_string(),
        },
        AvailableModel {
            id: "openai/gpt-3.5-turbo".to_string(),
            name: "GPT-3.5 Turbo".to_string(),
            provider: "OpenAI".to_string(),
        },
        AvailableModel {
            id: "anthropic/claude-sonnet-4.5".to_string(),
            name: "Claude Sonnet 4.5".to_string(),
            provider: "Anthropic".to_string(),
        },
        AvailableModel {
            id: "anthropic/claude-haiku-4.5".to_string(),
            name: "Claude Haiku 4.5".to_string(),
            provider: "Anthropic".to_string(),
        },
        AvailableModel {
            id: "anthropic/claude-opus-4.5".to_string(),
            name: "Claude Opus 4.5".to_string(),
            provider: "Anthropic".to_string(),
        },
        AvailableModel {
            id: "google/gemini-2-flash".to_string(),
            name: "Gemini 2 Flash".to_string(),
            provider: "Google".to_string(),
        },
        AvailableModel {
            id: "google/gemini-3-flash".to_string(),
            name: "Gemini 3 Flash".to_string(),
            provider: "Google".to_string(),
        },
        AvailableModel {
            id: "xai/grok-code-fast-1".to_string(),
            name: "Grok Code Fast".to_string(),
            provider: "xAI".to_string(),
        },
    ];
    debug!("[chat] returning {} models", result.len());
    result
}

/*
// Database-specific chat functionality removed
// The following commands are not included in the boilerplate:
// - create_chat_session_command
// - get_chat_session_command
// - list_chat_sessions_command
// - delete_chat_session_command
// - toggle_chat_bookmark_command
// - update_chat_title_command
// - create_chat_tag_command
// - list_chat_tags_command
// - delete_chat_tag_command
// - add_tag_to_session_command
// - remove_tag_from_session_command
// - database_chat_command
//
// Re-implement these if you need database-backed chat functionality
*/
