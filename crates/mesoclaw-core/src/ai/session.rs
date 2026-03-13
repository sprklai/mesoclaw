use crate::ai::adapter::{ToolCallEvent, ToolCallPhase};
use crate::db::{self, DbPool};
use crate::{MesoError, Result};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct Session {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_key: Option<String>,
}

fn default_source() -> String {
    "web".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ToolCallRecord {
    pub id: String,
    pub message_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub output: Option<String>,
    pub success: Option<bool>,
    pub duration_ms: Option<u64>,
    pub created_at: String,
}

pub struct SessionManager {
    db: DbPool,
}

impl SessionManager {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn create_session(&self, title: &str) -> Result<Session> {
        self.create_session_with_source(title, "web").await
    }

    pub async fn create_session_with_source(&self, title: &str, source: &str) -> Result<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let title = title.to_string();
        let source = source.to_string();

        let session_id = id.clone();
        let session_title = title.clone();
        let session_now = now.clone();
        let session_source = source.clone();

        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO sessions (id, title, created_at, updated_at, source) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![session_id, session_title, session_now, session_now, session_source],
            )?;
            Ok(())
        })
        .await?;

        Ok(Session {
            id,
            title,
            created_at: now.clone(),
            updated_at: now,
            source,
            channel_key: None,
        })
    }

    pub async fn create_session_with_channel_key(
        &self,
        title: &str,
        source: &str,
        channel_key: &str,
    ) -> Result<Session> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let title = title.to_string();
        let source = source.to_string();
        let channel_key = channel_key.to_string();

        let session_id = id.clone();
        let session_title = title.clone();
        let session_now = now.clone();
        let session_source = source.clone();
        let session_ck = channel_key.clone();

        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO sessions (id, title, created_at, updated_at, source, channel_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![session_id, session_title, session_now, session_now, session_source, session_ck],
            )?;
            Ok(())
        })
        .await?;

        Ok(Session {
            id,
            title,
            created_at: now.clone(),
            updated_at: now,
            source,
            channel_key: Some(channel_key),
        })
    }

    pub async fn find_session_by_channel_key(&self, channel_key: &str) -> Result<Option<Session>> {
        let channel_key = channel_key.to_string();

        db::with_db(&self.db, move |conn| {
            let result = conn.query_row(
                "SELECT id, title, created_at, updated_at, source, channel_key FROM sessions WHERE channel_key = ?1",
                rusqlite::params![channel_key],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        source: row.get(4)?,
                        channel_key: row.get(5)?,
                    })
                },
            );
            match result {
                Ok(session) => Ok(Some(session)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(MesoError::Sqlite(e)),
            }
        })
        .await
    }

    pub async fn list_channel_sessions(
        &self,
        source: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<SessionSummary>> {
        let source = source.map(|s| s.to_string());

        db::with_db(&self.db, move |conn| {
            let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match source {
                Some(ref src) => (
                    "SELECT s.id, s.title, s.created_at, s.updated_at, COUNT(m.id), s.source, s.channel_key
                     FROM sessions s
                     LEFT JOIN messages m ON m.session_id = s.id
                     WHERE s.source != 'web' AND s.source = ?1
                     GROUP BY s.id
                     ORDER BY s.updated_at DESC
                     LIMIT ?2 OFFSET ?3".to_string(),
                    vec![
                        Box::new(src.clone()) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(limit as i64),
                        Box::new(offset as i64),
                    ],
                ),
                None => (
                    "SELECT s.id, s.title, s.created_at, s.updated_at, COUNT(m.id), s.source, s.channel_key
                     FROM sessions s
                     LEFT JOIN messages m ON m.session_id = s.id
                     WHERE s.source != 'web'
                     GROUP BY s.id
                     ORDER BY s.updated_at DESC
                     LIMIT ?1 OFFSET ?2".to_string(),
                    vec![
                        Box::new(limit as i64) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(offset as i64),
                    ],
                ),
            };

            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt
                .query_map(param_refs.as_slice(), |row| {
                    Ok(SessionSummary {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        message_count: row.get(4)?,
                        source: row.get(5)?,
                        channel_key: row.get(6)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(rows)
        })
        .await
    }

    pub async fn get_messages_paginated(
        &self,
        session_id: &str,
        limit: usize,
        before_id: Option<&str>,
    ) -> Result<Vec<Message>> {
        let session_id = session_id.to_string();
        let before_id = before_id.map(|s| s.to_string());

        db::with_db(&self.db, move |conn| {
            match before_id {
                Some(ref bid) => {
                    let mut stmt = conn.prepare(
                        "SELECT id, session_id, role, content, created_at
                         FROM messages
                         WHERE session_id = ?1
                           AND created_at < (SELECT created_at FROM messages WHERE id = ?2)
                         ORDER BY created_at DESC
                         LIMIT ?3",
                    )?;

                    let rows = stmt
                        .query_map(rusqlite::params![session_id, bid, limit as i64], |row| {
                            Ok(Message {
                                id: row.get(0)?,
                                session_id: row.get(1)?,
                                role: row.get(2)?,
                                content: row.get(3)?,
                                created_at: row.get(4)?,
                            })
                        })?
                        .collect::<std::result::Result<Vec<_>, _>>()?;

                    // Reverse to return in chronological order
                    let mut rows = rows;
                    rows.reverse();
                    Ok(rows)
                }
                None => {
                    let mut stmt = conn.prepare(
                        "SELECT id, session_id, role, content, created_at
                         FROM messages
                         WHERE session_id = ?1
                         ORDER BY created_at DESC
                         LIMIT ?2",
                    )?;

                    let rows = stmt
                        .query_map(rusqlite::params![session_id, limit as i64], |row| {
                            Ok(Message {
                                id: row.get(0)?,
                                session_id: row.get(1)?,
                                role: row.get(2)?,
                                content: row.get(3)?,
                                created_at: row.get(4)?,
                            })
                        })?
                        .collect::<std::result::Result<Vec<_>, _>>()?;

                    let mut rows = rows;
                    rows.reverse();
                    Ok(rows)
                }
            }
        })
        .await
    }

    pub async fn get_session(&self, id: &str) -> Result<Session> {
        let id = id.to_string();

        db::with_db(&self.db, move |conn| {
            conn.query_row(
                "SELECT id, title, created_at, updated_at, source, channel_key FROM sessions WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        source: row.get(4)?,
                        channel_key: row.get(5)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    MesoError::NotFound(format!("session not found: {id}"))
                }
                other => MesoError::Sqlite(other),
            })
        })
        .await
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        db::with_db(&self.db, |conn| {
            let mut stmt = conn.prepare(
                "SELECT s.id, s.title, s.created_at, s.updated_at, COUNT(m.id) as message_count, s.source, s.channel_key
                 FROM sessions s
                 LEFT JOIN messages m ON m.session_id = s.id
                 GROUP BY s.id
                 ORDER BY s.updated_at DESC",
            )?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(SessionSummary {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        message_count: row.get(4)?,
                        source: row.get(5)?,
                        channel_key: row.get(6)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(rows)
        })
        .await
    }

    pub async fn update_session(&self, id: &str, title: &str) -> Result<Session> {
        let id = id.to_string();
        let title = title.to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let update_id = id.clone();
        let update_title = title.clone();
        let update_now = now.clone();

        db::with_db(&self.db, move |conn| {
            let rows_affected = conn.execute(
                "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![update_title, update_now, update_id],
            )?;

            if rows_affected == 0 {
                return Err(MesoError::NotFound(format!(
                    "session not found: {update_id}"
                )));
            }

            conn.query_row(
                "SELECT id, title, created_at, updated_at, source, channel_key FROM sessions WHERE id = ?1",
                rusqlite::params![update_id],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                        source: row.get(4)?,
                        channel_key: row.get(5)?,
                    })
                },
            )
            .map_err(MesoError::from)
        })
        .await
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        let id = id.to_string();

        db::with_db(&self.db, move |conn| {
            let rows_affected =
                conn.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![id])?;

            if rows_affected == 0 {
                return Err(MesoError::NotFound(format!("session not found: {id}")));
            }

            Ok(())
        })
        .await
    }

    pub async fn append_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<Message> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let session_id = session_id.to_string();
        let role = role.to_string();
        let content = content.to_string();

        let msg_id = id.clone();
        let msg_session_id = session_id.clone();
        let msg_role = role.clone();
        let msg_content = content.clone();
        let msg_now = now.clone();

        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO messages (id, session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![msg_id, msg_session_id, msg_role, msg_content, msg_now],
            )?;
            // Update session's updated_at timestamp whenever a message is appended
            conn.execute(
                "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
                rusqlite::params![msg_now, msg_session_id],
            )?;
            Ok(())
        })
        .await?;

        Ok(Message {
            id,
            session_id,
            role,
            content,
            created_at: now,
        })
    }

    /// Store tool call events linked to an assistant message.
    pub async fn store_tool_calls(
        &self,
        message_id: &str,
        session_id: &str,
        events: &[ToolCallEvent],
    ) -> Result<()> {
        // Pair Started+Completed events by call_id to build complete records
        let mut started: std::collections::HashMap<String, serde_json::Value> =
            std::collections::HashMap::new();
        // (call_id, tool_name, args_json, _uuid, output, success, duration_ms)
        type ToolRecord = (
            String,
            String,
            String,
            String,
            Option<String>,
            Option<bool>,
            Option<u64>,
        );
        let mut records: Vec<ToolRecord> = Vec::new();

        for evt in events {
            match &evt.phase {
                ToolCallPhase::Started { args } => {
                    started.insert(evt.call_id.clone(), args.clone());
                }
                ToolCallPhase::Completed {
                    output,
                    success,
                    duration_ms,
                } => {
                    let args = started
                        .remove(&evt.call_id)
                        .unwrap_or(serde_json::Value::Null);
                    records.push((
                        evt.call_id.clone(),
                        evt.tool_name.clone(),
                        serde_json::to_string(&args).unwrap_or_default(),
                        uuid::Uuid::new_v4().to_string(),
                        Some(output.clone()),
                        Some(*success),
                        Some(*duration_ms),
                    ));
                }
                ToolCallPhase::Cached { .. } => {
                    // Cached results are dedup replays — don't persist as separate records
                }
            }
        }

        // Also store any Started events that never got a Completed (shouldn't happen but be safe)
        for (call_id, args) in started {
            let tool_name = events
                .iter()
                .find(|e| e.call_id == call_id)
                .map(|e| e.tool_name.clone())
                .unwrap_or_default();
            records.push((
                call_id,
                tool_name,
                serde_json::to_string(&args).unwrap_or_default(),
                uuid::Uuid::new_v4().to_string(),
                None,
                None,
                None,
            ));
        }

        let message_id = message_id.to_string();
        let session_id = session_id.to_string();

        db::with_db(&self.db, move |conn| {
            for (call_id, tool_name, args, _uuid, output, success, duration_ms) in &records {
                conn.execute(
                    "INSERT INTO tool_calls (id, message_id, session_id, tool_name, args, output, success, duration_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params![
                        call_id,
                        message_id,
                        session_id,
                        tool_name,
                        args,
                        output,
                        success.map(|b| b as i32),
                        duration_ms.map(|d| d as i64),
                    ],
                )?;
            }
            Ok(())
        })
        .await
    }

    /// Retrieve tool call records for a given message.
    pub async fn get_tool_calls(&self, message_id: &str) -> Result<Vec<ToolCallRecord>> {
        let message_id = message_id.to_string();

        db::with_db(&self.db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, message_id, session_id, tool_name, args, output, success, duration_ms, created_at
                 FROM tool_calls
                 WHERE message_id = ?1
                 ORDER BY created_at ASC",
            )?;

            let rows = stmt
                .query_map(rusqlite::params![message_id], |row| {
                    let args_str: String = row.get(4)?;
                    let args: serde_json::Value =
                        serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Null);
                    let success_int: Option<i32> = row.get(6)?;
                    let duration: Option<i64> = row.get(7)?;

                    Ok(ToolCallRecord {
                        id: row.get(0)?,
                        message_id: row.get(1)?,
                        session_id: row.get(2)?,
                        tool_name: row.get(3)?,
                        args,
                        output: row.get(5)?,
                        success: success_int.map(|i| i != 0),
                        duration_ms: duration.map(|d| d as u64),
                        created_at: row.get(8)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(rows)
        })
        .await
    }

    /// Get the conversation summary for a session.
    pub async fn get_summary(&self, session_id: &str) -> Result<Option<String>> {
        let session_id = session_id.to_string();
        db::with_db(&self.db, move |conn| {
            let summary: Option<String> = conn
                .query_row(
                    "SELECT summary FROM sessions WHERE id = ?1",
                    rusqlite::params![session_id],
                    |row| row.get(0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        MesoError::NotFound(format!("session not found: {session_id}"))
                    }
                    other => MesoError::Sqlite(other),
                })?;
            Ok(summary)
        })
        .await
    }

    /// Set the conversation summary for a session.
    pub async fn set_summary(&self, session_id: &str, summary: &str) -> Result<()> {
        let session_id = session_id.to_string();
        let summary = summary.to_string();
        db::with_db(&self.db, move |conn| {
            let rows = conn
                .execute(
                    "UPDATE sessions SET summary = ?1 WHERE id = ?2",
                    rusqlite::params![summary, session_id],
                )
                .map_err(MesoError::from)?;
            if rows == 0 {
                return Err(MesoError::NotFound(format!(
                    "session not found: {session_id}"
                )));
            }
            Ok(())
        })
        .await
    }

    /// Get context-relevant info for a session: message count, last message time, summary.
    pub async fn get_context_info(
        &self,
        session_id: &str,
    ) -> Result<(usize, Option<chrono::DateTime<chrono::Utc>>, Option<String>)> {
        let session_id = session_id.to_string();
        db::with_db(&self.db, move |conn| {
            let (count, last_at): (i64, Option<String>) = conn
                .query_row(
                    "SELECT COUNT(*), MAX(created_at) FROM messages WHERE session_id = ?1",
                    rusqlite::params![session_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(MesoError::from)?;

            let summary: Option<String> = conn
                .query_row(
                    "SELECT summary FROM sessions WHERE id = ?1",
                    rusqlite::params![session_id],
                    |row| row.get(0),
                )
                .unwrap_or(None);

            let last_message_at = last_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            });

            Ok((count as usize, last_message_at, summary))
        })
        .await
    }

    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let session_id = session_id.to_string();

        db::with_db(&self.db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, role, content, created_at
                 FROM messages
                 WHERE session_id = ?1
                 ORDER BY created_at ASC",
            )?;

            let rows = stmt
                .query_map(rusqlite::params![session_id], |row| {
                    Ok(Message {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        role: row.get(2)?,
                        content: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            Ok(rows)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, SessionManager) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = db::init_pool(&path).unwrap();
        db::with_db(&pool, |conn| db::run_migrations(conn))
            .await
            .unwrap();
        let manager = SessionManager::new(pool);
        (dir, manager)
    }

    // 1.2.1 — create session
    #[tokio::test]
    async fn create_session() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("My Chat").await.unwrap();

        assert_eq!(session.title, "My Chat");
        assert!(!session.id.is_empty());
        assert!(!session.created_at.is_empty());
        assert!(!session.updated_at.is_empty());
    }

    // 1.2.2 — get session
    #[tokio::test]
    async fn get_session() {
        let (_dir, mgr) = setup().await;
        let created = mgr.create_session("Test").await.unwrap();
        let fetched = mgr.get_session(&created.id).await.unwrap();

        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.title, "Test");
    }

    // 1.2.3 — get session not found
    #[tokio::test]
    async fn get_session_not_found() {
        let (_dir, mgr) = setup().await;
        let result = mgr.get_session("nonexistent-id").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::NotFound(_)));
    }

    // 1.2.4 — list sessions empty
    #[tokio::test]
    async fn list_sessions_empty() {
        let (_dir, mgr) = setup().await;
        let sessions = mgr.list_sessions().await.unwrap();

        assert!(sessions.is_empty());
    }

    // 1.2.5 — list all sessions
    #[tokio::test]
    async fn list_sessions_all() {
        let (_dir, mgr) = setup().await;
        mgr.create_session("First").await.unwrap();
        mgr.create_session("Second").await.unwrap();

        let sessions = mgr.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        // Ordered by updated_at DESC, so "Second" first (no messages appended, both at creation time)
        assert_eq!(sessions[0].title, "Second");
        assert_eq!(sessions[1].title, "First");
        assert_eq!(sessions[0].message_count, 0);
    }

    // 1.2.6 — update session
    #[tokio::test]
    async fn update_session() {
        let (_dir, mgr) = setup().await;
        let created = mgr.create_session("Old Title").await.unwrap();
        let updated = mgr.update_session(&created.id, "New Title").await.unwrap();

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.created_at, created.created_at);
    }

    // 1.2.7 — delete session
    #[tokio::test]
    async fn delete_session() {
        let (_dir, mgr) = setup().await;
        let created = mgr.create_session("To Delete").await.unwrap();
        mgr.delete_session(&created.id).await.unwrap();

        let result = mgr.get_session(&created.id).await;
        assert!(matches!(result.unwrap_err(), MesoError::NotFound(_)));
    }

    // 1.2.8 — delete session not found
    #[tokio::test]
    async fn delete_session_not_found() {
        let (_dir, mgr) = setup().await;
        let result = mgr.delete_session("nonexistent-id").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::NotFound(_)));
    }

    // 1.2.9 — append message
    #[tokio::test]
    async fn append_message() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "user", "Hello!")
            .await
            .unwrap();

        assert_eq!(msg.session_id, session.id);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello!");
        assert!(!msg.id.is_empty());
    }

    // 1.2.10 — get messages ordered
    #[tokio::test]
    async fn get_messages_ordered() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        mgr.append_message(&session.id, "user", "First")
            .await
            .unwrap();
        mgr.append_message(&session.id, "assistant", "Second")
            .await
            .unwrap();
        mgr.append_message(&session.id, "user", "Third")
            .await
            .unwrap();

        let messages = mgr.get_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
        assert_eq!(messages[2].content, "Third");
    }

    // 1.2.11 — get messages empty
    #[tokio::test]
    async fn get_messages_empty() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Empty").await.unwrap();
        let messages = mgr.get_messages(&session.id).await.unwrap();

        assert!(messages.is_empty());
    }

    // 1.2.12 — append message to invalid session (FK constraint)
    #[tokio::test]
    async fn append_message_invalid_session() {
        let (_dir, mgr) = setup().await;
        let result = mgr
            .append_message("nonexistent-session", "user", "Hello")
            .await;

        assert!(result.is_err());
    }

    // TV.19 — store_tool_calls inserts records linked to message
    #[tokio::test]
    async fn store_tool_calls_inserts() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "assistant", "Using tools...")
            .await
            .unwrap();

        let events = vec![
            ToolCallEvent {
                call_id: "tc-1".into(),
                tool_name: "WebSearch".into(),
                phase: ToolCallPhase::Started {
                    args: serde_json::json!({"query": "rust"}),
                },
            },
            ToolCallEvent {
                call_id: "tc-1".into(),
                tool_name: "WebSearch".into(),
                phase: ToolCallPhase::Completed {
                    output: "results found".into(),
                    success: true,
                    duration_ms: 150,
                },
            },
        ];

        mgr.store_tool_calls(&msg.id, &session.id, &events)
            .await
            .unwrap();

        let records = mgr.get_tool_calls(&msg.id).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tool_name, "WebSearch");
        assert_eq!(records[0].success, Some(true));
        assert_eq!(records[0].duration_ms, Some(150));
    }

    // TV.20 — get_tool_calls returns stored records for message
    #[tokio::test]
    async fn get_tool_calls_returns_stored() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "assistant", "Done")
            .await
            .unwrap();

        let events = vec![
            ToolCallEvent {
                call_id: "tc-a".into(),
                tool_name: "FileRead".into(),
                phase: ToolCallPhase::Started {
                    args: serde_json::json!({"path": "/tmp/test"}),
                },
            },
            ToolCallEvent {
                call_id: "tc-a".into(),
                tool_name: "FileRead".into(),
                phase: ToolCallPhase::Completed {
                    output: "file contents".into(),
                    success: true,
                    duration_ms: 5,
                },
            },
            ToolCallEvent {
                call_id: "tc-b".into(),
                tool_name: "Shell".into(),
                phase: ToolCallPhase::Started {
                    args: serde_json::json!({"command": "ls"}),
                },
            },
            ToolCallEvent {
                call_id: "tc-b".into(),
                tool_name: "Shell".into(),
                phase: ToolCallPhase::Completed {
                    output: "error".into(),
                    success: false,
                    duration_ms: 10,
                },
            },
        ];

        mgr.store_tool_calls(&msg.id, &session.id, &events)
            .await
            .unwrap();

        let records = mgr.get_tool_calls(&msg.id).await.unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].tool_name, "FileRead");
        assert_eq!(records[1].tool_name, "Shell");
        assert_eq!(records[1].success, Some(false));
    }

    // TV.21 — get_tool_calls for message with no tools returns empty
    #[tokio::test]
    async fn get_tool_calls_empty() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "assistant", "No tools")
            .await
            .unwrap();

        let records = mgr.get_tool_calls(&msg.id).await.unwrap();
        assert!(records.is_empty());
    }

    // TV.22 — Deleting message cascades to tool_calls
    #[tokio::test]
    async fn delete_message_cascades_tool_calls() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "assistant", "Using tools")
            .await
            .unwrap();

        let events = vec![
            ToolCallEvent {
                call_id: "tc-del".into(),
                tool_name: "Shell".into(),
                phase: ToolCallPhase::Started {
                    args: serde_json::json!({}),
                },
            },
            ToolCallEvent {
                call_id: "tc-del".into(),
                tool_name: "Shell".into(),
                phase: ToolCallPhase::Completed {
                    output: "ok".into(),
                    success: true,
                    duration_ms: 1,
                },
            },
        ];

        mgr.store_tool_calls(&msg.id, &session.id, &events)
            .await
            .unwrap();

        // Delete the session (cascades to messages, which cascades to tool_calls)
        mgr.delete_session(&session.id).await.unwrap();

        let records = mgr.get_tool_calls(&msg.id).await.unwrap();
        assert!(records.is_empty());
    }

    // 15.3b — set and get session summary
    #[tokio::test]
    async fn session_set_and_get_summary() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();

        mgr.set_summary(
            &session.id,
            "Discussed Rust async patterns and error handling",
        )
        .await
        .unwrap();

        let summary = mgr.get_summary(&session.id).await.unwrap();
        assert_eq!(
            summary.as_deref(),
            Some("Discussed Rust async patterns and error handling")
        );
    }

    // 15.3b — summary is None when not set
    #[tokio::test]
    async fn session_summary_null_when_not_set() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();

        let summary = mgr.get_summary(&session.id).await.unwrap();
        assert!(summary.is_none());
    }

    // CR.28 — create_session with source stores correct value
    #[tokio::test]
    async fn create_session_with_source() {
        let (_dir, mgr) = setup().await;
        let session = mgr
            .create_session_with_source("Telegram Chat", "telegram")
            .await
            .unwrap();
        assert_eq!(session.source, "telegram");
    }

    // CR.29 — create_session defaults source to "web" when not specified
    #[tokio::test]
    async fn create_session_default_source() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Web Chat").await.unwrap();
        assert_eq!(session.source, "web");
    }

    // CR.30 — get_session returns source field
    #[tokio::test]
    async fn get_session_includes_source() {
        let (_dir, mgr) = setup().await;
        let created = mgr
            .create_session_with_source("Slack Chat", "slack")
            .await
            .unwrap();
        let fetched = mgr.get_session(&created.id).await.unwrap();
        assert_eq!(fetched.source, "slack");
    }

    // IN.1 — create_session_with_channel_key stores channel_key
    #[tokio::test]
    async fn create_session_with_channel_key() {
        let (_dir, mgr) = setup().await;
        let session = mgr
            .create_session_with_channel_key("Telegram #12345", "telegram", "telegram:12345")
            .await
            .unwrap();
        assert_eq!(session.channel_key.as_deref(), Some("telegram:12345"));
        assert_eq!(session.source, "telegram");
        assert_eq!(session.title, "Telegram #12345");
    }

    // IN.2 — find_session_by_channel_key returns existing session
    #[tokio::test]
    async fn find_session_by_channel_key_found() {
        let (_dir, mgr) = setup().await;
        let created = mgr
            .create_session_with_channel_key("Telegram #111", "telegram", "telegram:111")
            .await
            .unwrap();
        let found = mgr
            .find_session_by_channel_key("telegram:111")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);
    }

    // IN.3 — find_session_by_channel_key returns None for unknown key
    #[tokio::test]
    async fn find_session_by_channel_key_not_found() {
        let (_dir, mgr) = setup().await;
        let found = mgr
            .find_session_by_channel_key("telegram:999")
            .await
            .unwrap();
        assert!(found.is_none());
    }

    // IN.4 — list_channel_sessions excludes web sessions
    #[tokio::test]
    async fn list_channel_sessions_excludes_web() {
        let (_dir, mgr) = setup().await;
        mgr.create_session("Web Chat").await.unwrap();
        mgr.create_session_with_channel_key("TG Chat", "telegram", "telegram:1")
            .await
            .unwrap();
        mgr.create_session_with_channel_key("Slack Chat", "slack", "slack:C1")
            .await
            .unwrap();

        let all = mgr.list_channel_sessions(None, 50, 0).await.unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|s| s.source != "web"));
    }

    // IN.5 — list_channel_sessions filters by source
    #[tokio::test]
    async fn list_channel_sessions_filter_source() {
        let (_dir, mgr) = setup().await;
        mgr.create_session_with_channel_key("TG 1", "telegram", "telegram:1")
            .await
            .unwrap();
        mgr.create_session_with_channel_key("Slack 1", "slack", "slack:C1")
            .await
            .unwrap();

        let tg = mgr
            .list_channel_sessions(Some("telegram"), 50, 0)
            .await
            .unwrap();
        assert_eq!(tg.len(), 1);
        assert_eq!(tg[0].source, "telegram");
    }

    // IN.6 — get_messages_paginated returns latest N messages
    #[tokio::test]
    async fn get_messages_paginated_latest() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        for i in 0..5 {
            mgr.append_message(&session.id, "user", &format!("msg {i}"))
                .await
                .unwrap();
        }

        let msgs = mgr
            .get_messages_paginated(&session.id, 3, None)
            .await
            .unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].content, "msg 2");
        assert_eq!(msgs[2].content, "msg 4");
    }

    // IN.7 — get_messages_paginated cursor-based pagination
    #[tokio::test]
    async fn get_messages_paginated_cursor() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let mut msg_ids = vec![];
        for i in 0..5 {
            let m = mgr
                .append_message(&session.id, "user", &format!("msg {i}"))
                .await
                .unwrap();
            msg_ids.push(m.id);
        }

        // Get 2 messages before msg 3 (index 3)
        let msgs = mgr
            .get_messages_paginated(&session.id, 2, Some(&msg_ids[3]))
            .await
            .unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "msg 1");
        assert_eq!(msgs[1].content, "msg 2");
    }

    // WS-3.3 — append_message updates session updated_at
    #[tokio::test]
    async fn append_message_updates_session_timestamp() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("test").await.unwrap();
        let original_updated = session.updated_at.clone();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        mgr.append_message(&session.id, "user", "hello")
            .await
            .unwrap();
        let updated = mgr.get_session(&session.id).await.unwrap();
        assert!(
            updated.updated_at > original_updated,
            "updated_at should increase after append_message"
        );
    }

    // WS-3.4 — sessions ordered by updated_at (most recently updated first)
    #[tokio::test]
    async fn sessions_ordered_by_updated_at() {
        let (_dir, mgr) = setup().await;
        let s1 = mgr.create_session("first").await.unwrap();
        let _s2 = mgr.create_session("second").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        mgr.append_message(&s1.id, "user", "hello").await.unwrap();
        let sessions = mgr.list_sessions().await.unwrap();
        assert_eq!(
            sessions[0].id, s1.id,
            "Most recently updated should be first"
        );
    }

    // IN.8 — channel_key unique constraint prevents duplicates
    #[tokio::test]
    async fn channel_key_unique_constraint() {
        let (_dir, mgr) = setup().await;
        mgr.create_session_with_channel_key("TG 1", "telegram", "telegram:1")
            .await
            .unwrap();
        let result = mgr
            .create_session_with_channel_key("TG 1 dup", "telegram", "telegram:1")
            .await;
        assert!(result.is_err());
    }
}
