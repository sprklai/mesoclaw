use crate::ai::adapter::{ToolCallEvent, ToolCallPhase};
use crate::db::{self, DbPool};
use crate::{Result, ZeniiError};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRecord {
    pub delegation_id: String,
    pub total_duration_ms: u64,
    pub total_tokens: u64,
    pub agents: Vec<DelegationAgentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationAgentRecord {
    pub id: String,
    pub description: String,
    pub status: String,
    pub tool_uses: u32,
    pub tokens_used: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
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
                Err(e) => Err(ZeniiError::Sqlite(e)),
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
                    ZeniiError::NotFound(format!("session not found: {id}"))
                }
                other => ZeniiError::Sqlite(other),
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

    /// Delete sessions older than `max_age_days` and their associated messages/tool_calls.
    pub async fn cleanup_old_sessions(&self, max_age_days: u32) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(max_age_days));
        let cutoff_str = cutoff.to_rfc3339();

        db::with_db(&self.db, move |conn| {
            // Delete tool_calls for old messages
            conn.execute(
                "DELETE FROM tool_calls WHERE message_id IN (
                    SELECT m.id FROM messages m
                    JOIN sessions s ON m.session_id = s.id
                    WHERE s.updated_at < ?1
                )",
                rusqlite::params![cutoff_str],
            )?;
            // Delete messages for old sessions
            conn.execute(
                "DELETE FROM messages WHERE session_id IN (
                    SELECT id FROM sessions WHERE updated_at < ?1
                )",
                rusqlite::params![cutoff_str],
            )?;
            // Delete old sessions
            let deleted = conn.execute(
                "DELETE FROM sessions WHERE updated_at < ?1",
                rusqlite::params![cutoff_str],
            )?;
            Ok(deleted)
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
                return Err(ZeniiError::NotFound(format!(
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
            .map_err(ZeniiError::from)
        })
        .await
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        let id = id.to_string();

        db::with_db(&self.db, move |conn| {
            let rows_affected =
                conn.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![id])?;

            if rows_affected == 0 {
                return Err(ZeniiError::NotFound(format!("session not found: {id}")));
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
                ToolCallPhase::Cached { .. }
                | ToolCallPhase::ApprovalRequested { .. }
                | ToolCallPhase::ApprovalResolved { .. } => {
                    // Cached/approval events — don't persist as tool call records
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

    /// Store delegation task results linked to an assistant message.
    pub async fn store_delegation(
        &self,
        message_id: &str,
        session_id: &str,
        result: &crate::ai::delegation::task::DelegationResult,
    ) -> Result<()> {
        let db = self.db.clone();
        let message_id = message_id.to_string();
        let session_id = session_id.to_string();
        let delegation_id = result.id.clone();
        let total_duration_ms = result.total_duration_ms as i64;
        let total_tokens = result.total_usage.total_tokens as i64;

        // Collect task data before moving into spawn_blocking
        let tasks: Vec<_> = result
            .task_results
            .iter()
            .map(|t| {
                (
                    uuid::Uuid::new_v4().to_string(),
                    t.task_id.clone(),
                    t.description.clone(),
                    format!("{:?}", t.status),
                    t.tool_uses as i64,
                    t.usage.total_tokens as i64,
                    t.duration_ms as i64,
                    t.error.clone(),
                )
            })
            .collect();

        db::with_db(&db, move |conn| {
            for (id, agent_id, description, status, tool_uses, tokens_used, duration_ms, error) in
                &tasks
            {
                conn.execute(
                    "INSERT INTO delegation_tasks (id, message_id, session_id, delegation_id, agent_id, description, status, tool_uses, tokens_used, duration_ms, error, total_duration_ms, total_tokens)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    rusqlite::params![
                        id,
                        message_id,
                        session_id,
                        delegation_id,
                        agent_id,
                        description,
                        status,
                        tool_uses,
                        tokens_used,
                        duration_ms,
                        error,
                        total_duration_ms,
                        total_tokens
                    ],
                )?;
            }
            Ok(())
        })
        .await
    }

    /// Retrieve delegation record for a given message.
    pub async fn get_delegation(
        &self,
        message_id: &str,
    ) -> Result<Option<DelegationRecord>> {
        let db = self.db.clone();
        let message_id = message_id.to_string();

        db::with_db(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT delegation_id, agent_id, description, status, tool_uses, tokens_used, duration_ms, error, total_duration_ms, total_tokens
                 FROM delegation_tasks WHERE message_id = ?1 ORDER BY created_at ASC",
            )?;
            let rows: Vec<_> = stmt
                .query_map(rusqlite::params![message_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            if rows.is_empty() {
                return Ok(None);
            }

            let delegation_id = rows[0].0.clone();
            let total_duration_ms = rows[0].8 as u64;
            let total_tokens = rows[0].9 as u64;

            let agents = rows
                .into_iter()
                .map(|r| DelegationAgentRecord {
                    id: r.1,
                    description: r.2,
                    status: r.3,
                    tool_uses: r.4 as u32,
                    tokens_used: r.5 as u64,
                    duration_ms: r.6 as u64,
                    error: r.7,
                })
                .collect();

            Ok(Some(DelegationRecord {
                delegation_id,
                total_duration_ms,
                total_tokens,
                agents,
            }))
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
                        ZeniiError::NotFound(format!("session not found: {session_id}"))
                    }
                    other => ZeniiError::Sqlite(other),
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
                .map_err(ZeniiError::from)?;
            if rows == 0 {
                return Err(ZeniiError::NotFound(format!(
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
                .map_err(ZeniiError::from)?;

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

    /// Delete a message and all messages after it in the same session.
    /// Returns the number of deleted rows.
    pub async fn delete_messages_from(&self, session_id: &str, message_id: &str) -> Result<u64> {
        let session_id = session_id.to_string();
        let message_id = message_id.to_string();

        db::with_db(&self.db, move |conn| {
            // Find the created_at of the target message (must belong to this session)
            let created_at: String = conn
                .query_row(
                    "SELECT created_at FROM messages WHERE id = ?1 AND session_id = ?2",
                    rusqlite::params![message_id, session_id],
                    |row| row.get(0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        ZeniiError::NotFound(format!("message not found: {message_id}"))
                    }
                    other => ZeniiError::Sqlite(other),
                })?;

            // Delete tool_calls for messages that will be removed
            conn.execute(
                "DELETE FROM tool_calls WHERE message_id IN (
                    SELECT id FROM messages WHERE session_id = ?1 AND created_at >= ?2
                )",
                rusqlite::params![session_id, created_at],
            )?;

            // Delete the messages
            let deleted = conn.execute(
                "DELETE FROM messages WHERE session_id = ?1 AND created_at >= ?2",
                rusqlite::params![session_id, created_at],
            )?;

            Ok(deleted as u64)
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
        assert!(matches!(result.unwrap_err(), ZeniiError::NotFound(_)));
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
        assert!(matches!(result.unwrap_err(), ZeniiError::NotFound(_)));
    }

    // 1.2.8 — delete session not found
    #[tokio::test]
    async fn delete_session_not_found() {
        let (_dir, mgr) = setup().await;
        let result = mgr.delete_session("nonexistent-id").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ZeniiError::NotFound(_)));
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

    // AUDIT-H3.1 — cleanup_old_sessions with large max_age deletes nothing
    #[tokio::test]
    async fn cleanup_old_sessions_keeps_recent() {
        let (_dir, mgr) = setup().await;
        mgr.create_session("Recent 1").await.unwrap();
        mgr.create_session("Recent 2").await.unwrap();
        let deleted = mgr.cleanup_old_sessions(9999).await.unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(mgr.list_sessions().await.unwrap().len(), 2);
    }

    // AUDIT-H3.2 — cleanup_old_sessions with 0 days deletes all
    #[tokio::test]
    async fn cleanup_old_sessions_deletes_all() {
        let (_dir, mgr) = setup().await;
        let s = mgr.create_session("Old session").await.unwrap();
        mgr.append_message(&s.id, "user", "hello").await.unwrap();
        let deleted = mgr.cleanup_old_sessions(0).await.unwrap();
        assert_eq!(deleted, 1);
        assert!(mgr.list_sessions().await.unwrap().is_empty());
        // Messages should be cascade deleted too
        assert!(mgr.get_messages(&s.id).await.unwrap().is_empty());
    }

    // AUDIT-H3.3 — cleanup_old_sessions on empty DB returns 0
    #[tokio::test]
    async fn cleanup_old_sessions_empty_db() {
        let (_dir, mgr) = setup().await;
        let deleted = mgr.cleanup_old_sessions(30).await.unwrap();
        assert_eq!(deleted, 0);
    }

    // EDIT.1 — delete_messages_from removes target and later messages
    #[tokio::test]
    async fn delete_messages_from_success() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let _m1 = mgr
            .append_message(&session.id, "user", "First")
            .await
            .unwrap();
        let _m2 = mgr
            .append_message(&session.id, "assistant", "Second")
            .await
            .unwrap();
        let m3 = mgr
            .append_message(&session.id, "user", "Third")
            .await
            .unwrap();
        let _m4 = mgr
            .append_message(&session.id, "assistant", "Fourth")
            .await
            .unwrap();

        let deleted = mgr.delete_messages_from(&session.id, &m3.id).await.unwrap();
        assert_eq!(deleted, 2);

        let remaining = mgr.get_messages(&session.id).await.unwrap();
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].content, "First");
        assert_eq!(remaining[1].content, "Second");
    }

    // EDIT.2 — delete_messages_from first message empties conversation
    #[tokio::test]
    async fn delete_messages_from_first_message() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let m1 = mgr
            .append_message(&session.id, "user", "First")
            .await
            .unwrap();
        let _m2 = mgr
            .append_message(&session.id, "assistant", "Second")
            .await
            .unwrap();

        let deleted = mgr.delete_messages_from(&session.id, &m1.id).await.unwrap();
        assert_eq!(deleted, 2);

        let remaining = mgr.get_messages(&session.id).await.unwrap();
        assert!(remaining.is_empty());
    }

    // EDIT.3 — delete_messages_from with bad message_id returns NotFound
    #[tokio::test]
    async fn delete_messages_from_bad_message_id() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        mgr.append_message(&session.id, "user", "Hello")
            .await
            .unwrap();

        let result = mgr
            .delete_messages_from(&session.id, "nonexistent-msg")
            .await;
        assert!(matches!(result.unwrap_err(), ZeniiError::NotFound(_)));
    }

    // EDIT.4 — delete_messages_from with wrong session returns NotFound
    #[tokio::test]
    async fn delete_messages_from_wrong_session() {
        let (_dir, mgr) = setup().await;
        let session_a = mgr.create_session("A").await.unwrap();
        let session_b = mgr.create_session("B").await.unwrap();
        let msg_b = mgr
            .append_message(&session_b.id, "user", "In B")
            .await
            .unwrap();

        let result = mgr.delete_messages_from(&session_a.id, &msg_b.id).await;
        assert!(matches!(result.unwrap_err(), ZeniiError::NotFound(_)));
    }

    // EDIT.5 — delete_messages_from cascades to tool_calls
    #[tokio::test]
    async fn delete_messages_from_cascades_tool_calls() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let _m1 = mgr
            .append_message(&session.id, "user", "Do something")
            .await
            .unwrap();
        let m2 = mgr
            .append_message(&session.id, "assistant", "Using tools")
            .await
            .unwrap();

        let events = vec![
            ToolCallEvent {
                call_id: "tc-edit".into(),
                tool_name: "Shell".into(),
                phase: ToolCallPhase::Started {
                    args: serde_json::json!({"cmd": "ls"}),
                },
            },
            ToolCallEvent {
                call_id: "tc-edit".into(),
                tool_name: "Shell".into(),
                phase: ToolCallPhase::Completed {
                    output: "ok".into(),
                    success: true,
                    duration_ms: 5,
                },
            },
        ];
        mgr.store_tool_calls(&m2.id, &session.id, &events)
            .await
            .unwrap();

        // Delete from m2 (the assistant message with tool calls)
        mgr.delete_messages_from(&session.id, &m2.id).await.unwrap();

        // Tool calls should be gone
        let records = mgr.get_tool_calls(&m2.id).await.unwrap();
        assert!(records.is_empty());

        // Only m1 should remain
        let remaining = mgr.get_messages(&session.id).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].content, "Do something");
    }

    // DEL.1 — store_delegation and retrieve
    #[tokio::test]
    async fn store_delegation_and_retrieve() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "assistant", "delegation result")
            .await
            .unwrap();

        let delegation_result = crate::ai::delegation::task::DelegationResult {
            id: "del-1".into(),
            task_results: vec![
                crate::ai::delegation::task::TaskResult {
                    task_id: "agent-1".into(),
                    status: crate::ai::delegation::task::TaskStatus::Completed,
                    output: "result 1".into(),
                    usage: crate::ai::agent::TokenUsage {
                        input_tokens: 100,
                        output_tokens: 50,
                        total_tokens: 150,
                        cached_input_tokens: 0,
                    },
                    duration_ms: 1000,
                    error: None,
                    session_id: session.id.clone(),
                    tool_uses: 3,
                    description: "Research topic A".into(),
                },
                crate::ai::delegation::task::TaskResult {
                    task_id: "agent-2".into(),
                    status: crate::ai::delegation::task::TaskStatus::Completed,
                    output: "result 2".into(),
                    usage: crate::ai::agent::TokenUsage {
                        input_tokens: 200,
                        output_tokens: 100,
                        total_tokens: 300,
                        cached_input_tokens: 0,
                    },
                    duration_ms: 2000,
                    error: None,
                    session_id: session.id.clone(),
                    tool_uses: 5,
                    description: "Research topic B".into(),
                },
            ],
            aggregated_response: "combined".into(),
            total_usage: crate::ai::agent::TokenUsage {
                input_tokens: 300,
                output_tokens: 150,
                total_tokens: 450,
                cached_input_tokens: 0,
            },
            total_duration_ms: 2500,
        };

        mgr.store_delegation(&msg.id, &session.id, &delegation_result)
            .await
            .unwrap();

        let record = mgr.get_delegation(&msg.id).await.unwrap();
        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.delegation_id, "del-1");
        assert_eq!(record.total_duration_ms, 2500);
        assert_eq!(record.total_tokens, 450);
        assert_eq!(record.agents.len(), 2);
        assert_eq!(record.agents[0].id, "agent-1");
        assert_eq!(record.agents[0].description, "Research topic A");
        assert_eq!(record.agents[0].tool_uses, 3);
        assert_eq!(record.agents[1].id, "agent-2");
        assert_eq!(record.agents[1].description, "Research topic B");
    }

    // DEL.2 — get_delegation returns None for message without delegation
    #[tokio::test]
    async fn get_delegation_empty() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "assistant", "no delegation")
            .await
            .unwrap();

        let record = mgr.get_delegation(&msg.id).await.unwrap();
        assert!(record.is_none());
    }

    // DEL.3 — store_delegation preserves error field
    #[tokio::test]
    async fn store_delegation_preserves_error() {
        let (_dir, mgr) = setup().await;
        let session = mgr.create_session("Chat").await.unwrap();
        let msg = mgr
            .append_message(&session.id, "assistant", "partial failure")
            .await
            .unwrap();

        let delegation_result = crate::ai::delegation::task::DelegationResult {
            id: "del-2".into(),
            task_results: vec![
                crate::ai::delegation::task::TaskResult {
                    task_id: "agent-ok".into(),
                    status: crate::ai::delegation::task::TaskStatus::Completed,
                    output: "done".into(),
                    usage: crate::ai::agent::TokenUsage::default(),
                    duration_ms: 500,
                    error: None,
                    session_id: session.id.clone(),
                    tool_uses: 1,
                    description: "Successful task".into(),
                },
                crate::ai::delegation::task::TaskResult {
                    task_id: "agent-fail".into(),
                    status: crate::ai::delegation::task::TaskStatus::Failed,
                    output: String::new(),
                    usage: crate::ai::agent::TokenUsage::default(),
                    duration_ms: 200,
                    error: Some("connection refused".into()),
                    session_id: session.id.clone(),
                    tool_uses: 0,
                    description: "Failed task".into(),
                },
            ],
            aggregated_response: "partial".into(),
            total_usage: crate::ai::agent::TokenUsage::default(),
            total_duration_ms: 700,
        };

        mgr.store_delegation(&msg.id, &session.id, &delegation_result)
            .await
            .unwrap();

        let record = mgr.get_delegation(&msg.id).await.unwrap().unwrap();
        assert_eq!(record.agents.len(), 2);
        assert_eq!(record.agents[0].status, "Completed");
        assert!(record.agents[0].error.is_none());
        assert_eq!(record.agents[1].status, "Failed");
        assert_eq!(
            record.agents[1].error.as_deref(),
            Some("connection refused")
        );
    }
}
