use crate::Result;
use crate::db::{self, DbPool};

/// A known contact discovered from the sessions table.
#[derive(Debug, Clone)]
pub struct ChannelContact {
    /// Channel name (e.g. "telegram", "slack").
    pub channel: String,
    /// Recipient ID within the channel (e.g. "12345", "C123").
    pub recipient_id: String,
    /// Human-readable label (session title or fallback).
    pub label: String,
}

/// Query known contacts for a specific channel from the sessions table.
/// Uses `channel_key` column (format: "channel_name:recipient_id").
pub async fn query_channel_contacts(
    pool: &DbPool,
    channel_name: &str,
) -> Result<Vec<ChannelContact>> {
    let prefix = format!("{}:", channel_name);
    let channel_name_owned = channel_name.to_string();
    let pool = pool.clone();
    db::with_db(&pool, move |conn| {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT channel_key, title FROM sessions \
             WHERE channel_key LIKE ?1 AND channel_key IS NOT NULL \
             ORDER BY updated_at DESC LIMIT 20",
        )?;
        let rows = stmt.query_map([format!("{prefix}%")], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for row in rows {
            let (key, title) = row.map_err(crate::MesoError::from)?;
            let raw_id = key.strip_prefix(&prefix).unwrap_or(&key).to_string();
            // For Slack, deduplicate by extracting only the channel_id before any
            // second colon (thread_ts). e.g. "C123:1234567890.123456" -> "C123"
            let recipient_id = if channel_name_owned == "slack" {
                raw_id.split(':').next().unwrap_or(&raw_id).to_string()
            } else {
                raw_id
            };
            if !seen.insert(recipient_id.clone()) {
                continue;
            }
            let label = title.unwrap_or_else(|| format!("{}:{}", channel_name_owned, recipient_id));
            result.push(ChannelContact {
                channel: channel_name_owned.clone(),
                recipient_id,
                label,
            });
        }
        Ok(result)
    })
    .await
}

/// Query all known contacts across all channels from the sessions table.
pub async fn query_all_channel_contacts(pool: &DbPool) -> Result<Vec<ChannelContact>> {
    let pool = pool.clone();
    db::with_db(&pool, move |conn| {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT channel_key, title FROM sessions \
             WHERE channel_key IS NOT NULL \
             ORDER BY updated_at DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for row in rows {
            let (key, title) = row.map_err(crate::MesoError::from)?;
            // Split "channel:recipient_id" into parts
            let (channel, raw_id) = match key.split_once(':') {
                Some((ch, id)) => (ch.to_string(), id.to_string()),
                None => continue,
            };
            // For Slack, deduplicate by extracting only the channel_id before
            // any second colon (thread_ts).
            let recipient_id = if channel == "slack" {
                raw_id.split(':').next().unwrap_or(&raw_id).to_string()
            } else {
                raw_id
            };
            let dedup_key = format!("{}:{}", channel, recipient_id);
            if !seen.insert(dedup_key) {
                continue;
            }
            let label = title.unwrap_or_else(|| format!("{}:{}", channel, recipient_id));
            result.push(ChannelContact {
                channel,
                recipient_id,
                label,
            });
        }
        Ok(result)
    })
    .await
}

/// Count total known contacts across all channels.
pub async fn count_channel_contacts(pool: &DbPool) -> Result<usize> {
    let pool = pool.clone();
    db::with_db(&pool, move |conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT channel_key) FROM sessions WHERE channel_key IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::{Connection, params};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn setup_db() -> DbPool {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT,
                channel_key TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .expect("create table");
        Arc::new(Mutex::new(conn))
    }

    async fn insert_session(pool: &DbPool, id: &str, title: Option<&str>, channel_key: &str) {
        let id = id.to_string();
        let title = title.map(|t| t.to_string());
        let channel_key = channel_key.to_string();
        let pool = pool.clone();
        db::with_db(&pool, move |conn| {
            conn.execute(
                "INSERT INTO sessions (id, title, channel_key) VALUES (?1, ?2, ?3)",
                params![id, title, channel_key],
            )?;
            Ok(())
        })
        .await
        .expect("insert session");
    }

    #[tokio::test]
    async fn query_contacts_returns_known() {
        let pool = setup_db().await;
        insert_session(&pool, "s1", Some("Mario Chat"), "telegram:12345").await;
        insert_session(&pool, "s2", Some("DevTeam"), "telegram:-98765").await;

        let contacts = query_channel_contacts(&pool, "telegram").await.unwrap();
        assert_eq!(contacts.len(), 2);
        assert!(contacts.iter().any(|c| c.recipient_id == "12345"));
        assert!(contacts.iter().any(|c| c.recipient_id == "-98765"));
        assert!(contacts.iter().any(|c| c.label == "Mario Chat"));
    }

    #[tokio::test]
    async fn query_contacts_empty() {
        let pool = setup_db().await;
        let contacts = query_channel_contacts(&pool, "telegram").await.unwrap();
        assert!(contacts.is_empty());
    }

    #[tokio::test]
    async fn query_all_contacts_multiple() {
        let pool = setup_db().await;
        insert_session(&pool, "s1", Some("TG Chat"), "telegram:111").await;
        insert_session(&pool, "s2", Some("#general"), "slack:C222").await;

        let contacts = query_all_channel_contacts(&pool).await.unwrap();
        assert_eq!(contacts.len(), 2);
        assert!(
            contacts
                .iter()
                .any(|c| c.channel == "telegram" && c.recipient_id == "111")
        );
        assert!(
            contacts
                .iter()
                .any(|c| c.channel == "slack" && c.recipient_id == "C222")
        );
    }

    #[tokio::test]
    async fn slack_dedup_thread_ts() {
        let pool = setup_db().await;
        insert_session(&pool, "s1", Some("Thread 1"), "slack:C123:1234567890.123").await;
        insert_session(&pool, "s2", Some("Thread 2"), "slack:C123:9876543210.456").await;
        insert_session(&pool, "s3", Some("Other"), "slack:C999").await;

        let contacts = query_channel_contacts(&pool, "slack").await.unwrap();
        // C123 should appear only once (deduplicated), C999 separate
        assert_eq!(contacts.len(), 2);
        assert!(contacts.iter().any(|c| c.recipient_id == "C123"));
        assert!(contacts.iter().any(|c| c.recipient_id == "C999"));
    }
}
