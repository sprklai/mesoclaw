//! Database model for chat sessions.
//!
//! Sessions follow the structured key format: `{agent}:{scope}:{channel}:{peer}`.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::chat_sessions;

/// A chat session with structured routing metadata.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = chat_sessions)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: String,
    pub session_key: String,
    pub agent: String,
    pub scope: String,
    pub channel: String,
    pub peer: String,
    pub created_at: String,
    pub updated_at: String,
    pub compaction_summary: Option<String>,
}

/// Insertable chat session for creating new sessions.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = chat_sessions)]
pub struct NewChatSession {
    pub id: String,
    pub session_key: String,
    pub agent: String,
    pub scope: String,
    pub channel: String,
    pub peer: String,
    pub compaction_summary: Option<String>,
}

impl NewChatSession {
    /// Create a new session from a structured session key.
    pub fn from_session_key(key: &super::super::super::agent::session_router::SessionKey) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_key: key.as_str(),
            agent: key.agent.clone(),
            scope: key.scope.clone(),
            channel: key.channel.clone(),
            peer: key.peer.clone(),
            compaction_summary: None,
        }
    }
}

/// Updatable chat session fields.
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = chat_sessions)]
pub struct ChatSessionUpdate {
    pub updated_at: String,
    pub compaction_summary: Option<String>,
}
