//! Tauri IPC commands for chat session persistence.
//!
//! Provides CRUD operations for chat sessions and messages backed by SQLite.

use crate::database::DbPool;
use crate::database::models::chat_message::{
    ChatMessage, CreateSessionRequest, NewChatMessage, SaveMessageRequest,
};
use crate::database::models::chat_session::ChatSession;
use crate::database::schema::{chat_messages, chat_sessions};
use diesel::prelude::*;
use log::{debug, info};
use tauri::State;

/// List all chat sessions, ordered by most recently updated.
#[tauri::command]
pub fn list_chat_sessions_command(pool: State<'_, DbPool>) -> Result<Vec<ChatSession>, String> {
    info!("[chat_sessions] listing sessions");
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let sessions = chat_sessions::table
        .order(chat_sessions::updated_at.desc())
        .limit(50)
        .select(ChatSession::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())?;
    debug!("[chat_sessions] found {} sessions", sessions.len());
    Ok(sessions)
}

/// Get a single chat session by ID.
#[tauri::command]
pub fn get_chat_session_command(
    id: String,
    pool: State<'_, DbPool>,
) -> Result<ChatSession, String> {
    info!("[chat_sessions] getting session {}", id);
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    chat_sessions::table
        .find(&id)
        .select(ChatSession::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

/// Create a new chat session.
#[tauri::command]
pub fn create_chat_session_command(
    request: CreateSessionRequest,
    pool: State<'_, DbPool>,
) -> Result<ChatSession, String> {
    info!(
        "[chat_sessions] creating session for provider={}, model={}",
        request.provider_id, request.model_id
    );
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    let session_key = format!("chat:{}:{}", request.provider_id, request.model_id);

    diesel::insert_into(chat_sessions::table)
        .values((
            chat_sessions::id.eq(&id),
            chat_sessions::session_key.eq(&session_key),
            chat_sessions::agent.eq("main"),
            chat_sessions::scope.eq("dm"),
            chat_sessions::channel.eq("tauri"),
            chat_sessions::peer.eq("user"),
            chat_sessions::created_at.eq(&now),
            chat_sessions::updated_at.eq(&now),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    chat_sessions::table
        .find(&id)
        .select(ChatSession::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

/// Delete a chat session by ID.
#[tauri::command]
pub fn delete_chat_session_command(id: String, pool: State<'_, DbPool>) -> Result<(), String> {
    info!("[chat_sessions] deleting session {}", id);
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    diesel::delete(chat_sessions::table.find(&id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    debug!("[chat_sessions] deleted session {}", id);
    Ok(())
}

/// Load all messages for a session, ordered chronologically.
#[tauri::command]
pub fn load_messages_command(
    session_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<ChatMessage>, String> {
    info!(
        "[chat_sessions] loading messages for session {}",
        session_id
    );
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let messages = chat_messages::table
        .filter(chat_messages::session_id.eq(&session_id))
        .order(chat_messages::created_at.asc())
        .select(ChatMessage::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())?;
    debug!(
        "[chat_sessions] loaded {} messages for session {}",
        messages.len(),
        session_id
    );
    Ok(messages)
}

/// Save a new message to a session.
#[tauri::command]
pub fn save_message_command(
    request: SaveMessageRequest,
    pool: State<'_, DbPool>,
) -> Result<ChatMessage, String> {
    info!(
        "[chat_sessions] saving message for session {}, role={}",
        request.session_id, request.role
    );
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let created_at = chrono::Utc::now().to_rfc3339();

    let new_message = NewChatMessage::new(&request.session_id, &request.role, &request.content);

    diesel::insert_into(chat_messages::table)
        .values(&new_message)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    // Update session updated_at timestamp
    diesel::update(chat_sessions::table.filter(chat_sessions::id.eq(&new_message.session_id)))
        .set(chat_sessions::updated_at.eq(&created_at))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    chat_messages::table
        .find(&new_message.id)
        .select(ChatMessage::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

/// Clear all messages from a session.
#[tauri::command]
pub fn clear_session_messages_command(
    session_id: String,
    pool: State<'_, DbPool>,
) -> Result<(), String> {
    info!(
        "[chat_sessions] clearing messages for session {}",
        session_id
    );
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    diesel::delete(chat_messages::table.filter(chat_messages::session_id.eq(&session_id)))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    debug!(
        "[chat_sessions] cleared messages for session {}",
        session_id
    );
    Ok(())
}
