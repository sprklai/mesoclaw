//! Database model for chat messages.
//!
//! Messages belong to chat sessions and store the conversation history.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::chat_messages;

/// A chat message belonging to a session.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = chat_messages)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Insertable chat message for creating new messages.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = chat_messages)]
pub struct NewChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl NewChatMessage {
    /// Create a new message for a session.
    pub fn new(session_id: &str, role: &str, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Request payload for saving a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMessageRequest {
    pub session_id: String,
    pub role: String,
    pub content: String,
}

/// Request payload for creating a new session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub provider_id: String,
    pub model_id: String,
}
