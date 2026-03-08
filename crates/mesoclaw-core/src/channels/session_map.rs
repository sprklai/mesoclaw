use std::sync::Arc;

use dashmap::DashMap;

use crate::Result;

#[cfg(feature = "ai")]
use crate::ai::session::SessionManager;

use super::message::ChannelMessage;

/// Maps channel thread keys to session IDs, creating sessions on first contact.
pub struct ChannelSessionMap {
    map: DashMap<String, String>,
    #[cfg(feature = "ai")]
    session_manager: Arc<SessionManager>,
}

impl ChannelSessionMap {
    #[cfg(feature = "ai")]
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self {
            map: DashMap::new(),
            session_manager,
        }
    }

    /// Build a unique key from channel message metadata.
    ///
    /// Key format: `{channel}:{identifier}` where identifier depends on the channel:
    /// - telegram: `chat_id`
    /// - slack: `channel_id:thread_ts` (or just `channel_id` if no thread)
    /// - discord: `channel_id`
    /// - fallback: `sender` or "unknown"
    pub fn channel_key(message: &ChannelMessage) -> String {
        let channel = &message.channel;
        match channel.as_str() {
            "telegram" => {
                let chat_id = message
                    .metadata
                    .get("chat_id")
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                format!("telegram:{chat_id}")
            }
            "slack" => {
                let channel_id = message
                    .metadata
                    .get("channel_id")
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                if let Some(thread_ts) = message.metadata.get("thread_ts") {
                    format!("slack:{channel_id}:{thread_ts}")
                } else {
                    format!("slack:{channel_id}")
                }
            }
            "discord" => {
                let channel_id = message
                    .metadata
                    .get("channel_id")
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                format!("discord:{channel_id}")
            }
            other => {
                let sender = message.sender.as_deref().unwrap_or("unknown");
                format!("{other}:{sender}")
            }
        }
    }

    /// Resolve an existing session or create a new one for the given channel key.
    #[cfg(feature = "ai")]
    pub async fn resolve_session(&self, channel_key: &str, channel_name: &str) -> Result<String> {
        // Check existing mapping
        if let Some(session_id) = self.map.get(channel_key) {
            return Ok(session_id.clone());
        }

        // Create new session with source tag
        let title = format!("{} conversation", capitalize_first(channel_name));
        let session = self
            .session_manager
            .create_session_with_source(&title, channel_name)
            .await?;

        let session_id = session.id.clone();
        self.map.insert(channel_key.to_string(), session_id.clone());
        Ok(session_id)
    }

    /// List all active channel-to-session mappings.
    pub fn list_channel_sessions(&self) -> Vec<(String, String)> {
        self.map
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // CR.1 — channel_key builds correct key from telegram message metadata
    #[test]
    fn channel_key_telegram() {
        let mut meta = HashMap::new();
        meta.insert("chat_id".into(), "12345".into());
        let msg = ChannelMessage::new("telegram", "hello").with_metadata(meta);
        assert_eq!(ChannelSessionMap::channel_key(&msg), "telegram:12345");
    }

    // CR.2 — channel_key builds correct key from slack message with thread_ts
    #[test]
    fn channel_key_slack_thread() {
        let mut meta = HashMap::new();
        meta.insert("channel_id".into(), "C123".into());
        meta.insert("thread_ts".into(), "1234567890.123456".into());
        let msg = ChannelMessage::new("slack", "hello").with_metadata(meta);
        assert_eq!(
            ChannelSessionMap::channel_key(&msg),
            "slack:C123:1234567890.123456"
        );
    }

    // CR.3 — channel_key builds correct key from discord message
    #[test]
    fn channel_key_discord() {
        let mut meta = HashMap::new();
        meta.insert("channel_id".into(), "987654".into());
        let msg = ChannelMessage::new("discord", "hello").with_metadata(meta);
        assert_eq!(ChannelSessionMap::channel_key(&msg), "discord:987654");
    }

    // CR.4 — resolve_session creates new session on first message
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn resolve_creates_session() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&path).unwrap();
        crate::db::with_db(&pool, |conn| crate::db::run_migrations(conn))
            .await
            .unwrap();
        let mgr = Arc::new(crate::ai::session::SessionManager::new(pool));
        let map = ChannelSessionMap::new(mgr.clone());

        let session_id = map
            .resolve_session("telegram:12345", "telegram")
            .await
            .unwrap();
        assert!(!session_id.is_empty());

        // Verify session was created in DB
        let session = mgr.get_session(&session_id).await.unwrap();
        assert_eq!(session.title, "Telegram conversation");
        assert_eq!(session.source, "telegram");
    }

    // CR.5 — resolve_session returns same session_id for same channel_key
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn resolve_returns_existing() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&path).unwrap();
        crate::db::with_db(&pool, |conn| crate::db::run_migrations(conn))
            .await
            .unwrap();
        let mgr = Arc::new(crate::ai::session::SessionManager::new(pool));
        let map = ChannelSessionMap::new(mgr);

        let id1 = map
            .resolve_session("telegram:12345", "telegram")
            .await
            .unwrap();
        let id2 = map
            .resolve_session("telegram:12345", "telegram")
            .await
            .unwrap();
        assert_eq!(id1, id2);
    }

    // CR.6 — new session has correct source field
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn session_source_matches_channel() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&path).unwrap();
        crate::db::with_db(&pool, |conn| crate::db::run_migrations(conn))
            .await
            .unwrap();
        let mgr = Arc::new(crate::ai::session::SessionManager::new(pool));
        let map = ChannelSessionMap::new(mgr.clone());

        let session_id = map.resolve_session("slack:C123", "slack").await.unwrap();
        let session = mgr.get_session(&session_id).await.unwrap();
        assert_eq!(session.source, "slack");
    }

    // CR.7 — list_channel_sessions returns all active mappings
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn list_all_sessions() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&path).unwrap();
        crate::db::with_db(&pool, |conn| crate::db::run_migrations(conn))
            .await
            .unwrap();
        let mgr = Arc::new(crate::ai::session::SessionManager::new(pool));
        let map = ChannelSessionMap::new(mgr);

        map.resolve_session("telegram:111", "telegram")
            .await
            .unwrap();
        map.resolve_session("slack:C222", "slack").await.unwrap();

        let sessions = map.list_channel_sessions();
        assert_eq!(sessions.len(), 2);
    }
}
