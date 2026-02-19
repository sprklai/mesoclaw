//! Structured session key routing for the agent subsystem.
//!
//! # Session key format
//! ```text
//! {agent}:{scope}:{channel}:{peer}
//! ```
//! Examples:
//! - `main:dm:tauri:user`          — foreground chat with the user
//! - `main:cron:daily-report`      — scheduled daily report
//! - `main:heartbeat:check`        — periodic health check
//! - `isolated:task:analyze-db`    — isolated background task
//!
//! # Session isolation
//! Sessions with `scope = "isolated"` (i.e. `SessionKey::is_isolated()`) keep
//! a separate message history that does not bleed into the main chat.
//!
//! # Session compaction
//! [`SessionRouter::compact`] keeps the last `max_messages` messages and
//! prepends a one-line summary of dropped messages to prevent unbounded growth.

use std::{collections::HashMap, sync::RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── SessionKey ───────────────────────────────────────────────────────────────

/// A structured, namespaced session identifier.
///
/// Follows the format `{agent}:{scope}:{channel}:{peer}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionKey {
    pub agent: String,
    pub scope: String,
    pub channel: String,
    pub peer: String,
}

impl SessionKey {
    /// Parse a session key from its canonical string form.
    ///
    /// Returns `Err` if the string does not have exactly 4 colon-separated
    /// components.
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.splitn(4, ':').collect();
        if parts.len() != 4 {
            return Err(format!(
                "invalid session key '{s}': expected 4 colon-separated components"
            ));
        }
        Ok(Self {
            agent: parts[0].to_owned(),
            scope: parts[1].to_owned(),
            channel: parts[2].to_owned(),
            peer: parts[3].to_owned(),
        })
    }

    /// Return the canonical string representation.
    pub fn as_str(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.agent, self.scope, self.channel, self.peer
        )
    }

    /// Return `true` if this session is isolated (agent field is `"isolated"`).
    pub fn is_isolated(&self) -> bool {
        self.agent == "isolated"
    }

    /// The default foreground user session key.
    pub fn main_user() -> Self {
        Self {
            agent: "main".to_owned(),
            scope: "dm".to_owned(),
            channel: "tauri".to_owned(),
            peer: "user".to_owned(),
        }
    }

    /// Construct a key for a heartbeat check session.
    pub fn heartbeat() -> Self {
        Self {
            agent: "main".to_owned(),
            scope: "heartbeat".to_owned(),
            channel: "scheduler".to_owned(),
            peer: "check".to_owned(),
        }
    }

    /// Construct a key for an isolated background task.
    pub fn isolated_task(task: &str) -> Self {
        Self {
            agent: "isolated".to_owned(),
            scope: "task".to_owned(),
            channel: "scheduler".to_owned(),
            peer: task.to_owned(),
        }
    }

    /// Session key for a Telegram DM (positive chat ID).
    ///
    /// Each Telegram user gets a persistent session so the agent retains
    /// per-user context — mirrors OpenClaw's per-peer session isolation.
    pub fn telegram_dm(chat_id: i64) -> Self {
        Self {
            agent: "main".to_owned(),
            scope: "dm".to_owned(),
            channel: "telegram".to_owned(),
            peer: chat_id.to_string(),
        }
    }

    /// Session key for a Telegram group chat (negative chat ID).
    ///
    /// Group sessions are isolated so group chatter does not pollute the
    /// main DM context.
    pub fn telegram_group(chat_id: i64) -> Self {
        Self {
            agent: "isolated".to_owned(),
            scope: "group".to_owned(),
            channel: "telegram".to_owned(),
            peer: chat_id.to_string(),
        }
    }
}

impl std::fmt::Display for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ─── SessionMessage ───────────────────────────────────────────────────────────

/// A single turn in a session's message history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMessage {
    pub id: String,
    /// `"system"`, `"user"`, `"assistant"`, or `"tool_result"`.
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl SessionMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: role.into(),
            content: content.into(),
            created_at: Utc::now(),
        }
    }
}

// ─── Session ─────────────────────────────────────────────────────────────────

/// A conversation session with structured key and message history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub key: SessionKey,
    pub messages: Vec<SessionMessage>,
    pub created_at: DateTime<Utc>,
    /// Optional summary of compacted messages.
    pub compaction_summary: Option<String>,
}

impl Session {
    fn new(key: SessionKey) -> Self {
        Self {
            key,
            messages: Vec::new(),
            created_at: Utc::now(),
            compaction_summary: None,
        }
    }

    /// Append a message to the session history.
    pub fn push(&mut self, role: impl Into<String>, content: impl Into<String>) {
        self.messages.push(SessionMessage::new(role, content));
    }

    /// Number of messages in this session's history.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

// ─── SessionRouter ────────────────────────────────────────────────────────────

/// Routes channels and contexts to [`Session`] instances.
///
/// All sessions are held in memory.  Isolated sessions are stored separately
/// from main sessions to prevent history pollution.
///
/// The `chat_sessions` SQLite table (migration 2026-02-19-100000) stores
/// session metadata with columns: `session_key`, `agent`, `scope`, `channel`,
/// `peer`.  In-memory sessions are the source of truth at runtime; persistence
/// is a future follow-up.
pub struct SessionRouter {
    sessions: RwLock<HashMap<SessionKey, Session>>,
}

impl SessionRouter {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Resolve a `SessionKey` from a `channel` identifier and optional
    /// `context` string (e.g. job name for cron channels).
    ///
    /// | channel       | resulting key format                    |
    /// |---------------|-----------------------------------------|
    /// | `"user"`      | `main:dm:tauri:user`                    |
    /// | `"heartbeat"` | `main:heartbeat:scheduler:check`        |
    /// | `"cron"`      | `isolated:task:scheduler:{context}`     |
    /// | `"telegram"`  | `main:dm:telegram:{chat_id}` (DM)       |
    /// |               | `isolated:group:telegram:{chat_id}` (group) |
    /// | other         | `isolated:task:{channel}:{context}`     |
    pub fn resolve(&self, channel: &str, context: Option<&str>) -> SessionKey {
        let ctx = context.unwrap_or("default");
        match channel {
            "user" => SessionKey::main_user(),
            "heartbeat" => SessionKey::heartbeat(),
            "cron" => SessionKey {
                agent: "isolated".to_owned(),
                scope: "task".to_owned(),
                channel: "scheduler".to_owned(),
                peer: ctx.to_owned(),
            },
            // Telegram DMs get per-user persistent sessions; groups are isolated.
            // Context is expected to be the Telegram chat_id as a string.
            "telegram" => {
                let chat_id: i64 = ctx.parse().unwrap_or(0);
                if chat_id < 0 {
                    SessionKey::telegram_group(chat_id)
                } else {
                    SessionKey::telegram_dm(chat_id)
                }
            }
            other => SessionKey::isolated_task(&format!("{other}:{ctx}")),
        }
    }

    /// Create a new session for `key`.  If a session already exists it is
    /// returned unchanged (idempotent).
    pub fn create_session(&self, key: SessionKey) -> Result<(), String> {
        let mut map = self.sessions.write().map_err(|e| e.to_string())?;
        map.entry(key.clone()).or_insert_with(|| Session::new(key));
        Ok(())
    }

    /// Get a clone of the session identified by `key`, or `None`.
    pub fn get_session(&self, key: &SessionKey) -> Option<Session> {
        self.sessions.read().ok()?.get(key).cloned()
    }

    /// Ensure a session exists for `key`, creating it if necessary.
    pub fn get_or_create(&self, key: SessionKey) -> Result<Session, String> {
        {
            let map = self.sessions.read().map_err(|e| e.to_string())?;
            if let Some(s) = map.get(&key) {
                return Ok(s.clone());
            }
        }
        let session = Session::new(key.clone());
        self.sessions
            .write()
            .map_err(|e| e.to_string())?
            .insert(key, session.clone());
        Ok(session)
    }

    /// Push a message into the session identified by `key`.
    pub fn push_message(
        &self,
        key: &SessionKey,
        role: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<(), String> {
        let mut map = self.sessions.write().map_err(|e| e.to_string())?;
        let session = map
            .entry(key.clone())
            .or_insert_with(|| Session::new(key.clone()));
        session.push(role, content);
        Ok(())
    }

    /// Compact a session, keeping the last `max_messages` messages.
    ///
    /// Messages dropped from the front are recorded in `compaction_summary`
    /// so the agent knows some history was omitted.
    pub fn compact(&self, key: &SessionKey, max_messages: usize) -> Result<bool, String> {
        let mut map = self.sessions.write().map_err(|e| e.to_string())?;
        let Some(session) = map.get_mut(key) else {
            return Ok(false);
        };

        if session.messages.len() <= max_messages {
            return Ok(false); // Nothing to compact.
        }

        let drop_count = session.messages.len() - max_messages;
        let summary = format!(
            "[Compacted: {drop_count} earlier messages omitted. Session started at {}.]",
            session.created_at.format("%Y-%m-%d %H:%M UTC")
        );
        session.messages.drain(0..drop_count);
        session.compaction_summary = Some(summary);
        Ok(true)
    }

    /// List all session keys.
    pub fn list_keys(&self) -> Vec<SessionKey> {
        self.sessions
            .read()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Return the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.read().map(|m| m.len()).unwrap_or(0)
    }
}

impl Default for SessionRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn router() -> SessionRouter {
        SessionRouter::new()
    }

    // ─── SessionKey tests ─────────────────────────────────────────────────────

    #[test]
    fn parse_valid_session_key() {
        let key = SessionKey::parse("main:dm:tauri:user").unwrap();
        assert_eq!(key.agent, "main");
        assert_eq!(key.scope, "dm");
        assert_eq!(key.channel, "tauri");
        assert_eq!(key.peer, "user");
    }

    #[test]
    fn parse_invalid_session_key_errors() {
        assert!(
            SessionKey::parse("main:dm").is_err(),
            "too few parts → error"
        );
    }

    #[test]
    fn session_key_round_trip() {
        let original = "isolated:task:scheduler:daily-report";
        let key = SessionKey::parse(original).unwrap();
        assert_eq!(key.as_str(), original, "as_str should round-trip");
    }

    #[test]
    fn main_user_key_not_isolated() {
        let key = SessionKey::main_user();
        assert!(
            !key.is_isolated(),
            "main user session should not be isolated"
        );
    }

    #[test]
    fn isolated_task_key_is_isolated() {
        let key = SessionKey::isolated_task("analyze");
        assert!(key.is_isolated(), "isolated task key should be isolated");
    }

    // ─── SessionRouter tests ──────────────────────────────────────────────────

    #[test]
    fn resolve_user_channel() {
        let r = router();
        let key = r.resolve("user", None);
        assert_eq!(key, SessionKey::main_user());
    }

    #[test]
    fn resolve_heartbeat_channel() {
        let r = router();
        let key = r.resolve("heartbeat", None);
        assert_eq!(key, SessionKey::heartbeat());
    }

    #[test]
    fn resolve_cron_channel_with_context() {
        let r = router();
        let key = r.resolve("cron", Some("daily-report"));
        assert_eq!(key.peer, "daily-report");
        assert!(key.is_isolated(), "cron session should be isolated");
    }

    #[test]
    fn get_or_create_returns_empty_session() {
        let r = router();
        let key = SessionKey::main_user();
        let session = r.get_or_create(key.clone()).unwrap();
        assert!(session.is_empty(), "new session should have no messages");
        assert_eq!(session.key, key);
    }

    #[test]
    fn push_message_adds_to_history() {
        let r = router();
        let key = SessionKey::main_user();
        r.push_message(&key, "user", "Hello!").unwrap();
        r.push_message(&key, "assistant", "Hi there!").unwrap();

        let session = r.get_session(&key).unwrap();
        assert_eq!(session.len(), 2, "should have 2 messages");
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].role, "assistant");
    }

    #[test]
    fn isolated_sessions_stored_separately() {
        let r = router();
        let main_key = SessionKey::main_user();
        let isolated_key = SessionKey::isolated_task("task1");

        r.push_message(&main_key, "user", "Main chat").unwrap();
        r.push_message(&isolated_key, "user", "Isolated task")
            .unwrap();

        assert_eq!(r.session_count(), 2, "should have 2 sessions");
        let main = r.get_session(&main_key).unwrap();
        let isolated = r.get_session(&isolated_key).unwrap();
        assert_eq!(main.messages[0].content, "Main chat");
        assert_eq!(isolated.messages[0].content, "Isolated task");
    }

    #[test]
    fn compact_reduces_history() {
        let r = router();
        let key = SessionKey::main_user();
        for i in 0..20 {
            r.push_message(&key, "user", format!("msg {i}")).unwrap();
        }
        let compacted = r.compact(&key, 5).unwrap();
        assert!(compacted, "compaction should have occurred");

        let session = r.get_session(&key).unwrap();
        assert_eq!(
            session.len(),
            5,
            "should have only 5 messages after compaction"
        );
        assert!(
            session.compaction_summary.is_some(),
            "should have compaction summary"
        );
    }

    #[test]
    fn compact_no_op_when_within_limit() {
        let r = router();
        let key = SessionKey::main_user();
        r.push_message(&key, "user", "hello").unwrap();
        let compacted = r.compact(&key, 10).unwrap();
        assert!(!compacted, "no compaction needed when under limit");
    }

    #[test]
    fn compact_nonexistent_session_returns_false() {
        let r = router();
        let key = SessionKey::parse("x:y:z:w").unwrap();
        let result = r.compact(&key, 5).unwrap();
        assert!(!result, "compacting non-existent session → false");
    }

    #[test]
    fn list_keys_returns_all_sessions() {
        let r = router();
        let k1 = SessionKey::main_user();
        let k2 = SessionKey::heartbeat();
        r.get_or_create(k1).unwrap();
        r.get_or_create(k2).unwrap();
        let keys = r.list_keys();
        assert_eq!(keys.len(), 2);
    }

    // ── Telegram session keys (OpenClaw per-peer isolation pattern) ───────────

    #[test]
    fn telegram_dm_key_is_not_isolated() {
        let key = SessionKey::telegram_dm(123_456_789);
        assert!(!key.is_isolated());
        assert_eq!(key.channel, "telegram");
        assert_eq!(key.scope, "dm");
        assert_eq!(key.peer, "123456789");
    }

    #[test]
    fn telegram_group_key_is_isolated() {
        let key = SessionKey::telegram_group(-1_001_234_567_890);
        assert!(key.is_isolated());
        assert_eq!(key.channel, "telegram");
        assert_eq!(key.scope, "group");
    }

    #[test]
    fn resolve_telegram_dm_chat_id() {
        let r = router();
        let key = r.resolve("telegram", Some("123456789"));
        assert!(!key.is_isolated());
        assert_eq!(key.scope, "dm");
        assert_eq!(key.channel, "telegram");
        assert_eq!(key.peer, "123456789");
    }

    #[test]
    fn resolve_telegram_group_chat_id() {
        let r = router();
        let key = r.resolve("telegram", Some("-1001234567890"));
        assert!(key.is_isolated());
        assert_eq!(key.scope, "group");
    }

    #[test]
    fn separate_telegram_users_get_separate_sessions() {
        let r = router();
        let alice = r.resolve("telegram", Some("111"));
        let bob = r.resolve("telegram", Some("222"));
        r.push_message(&alice, "user", "alice msg").unwrap();
        r.push_message(&bob, "user", "bob msg").unwrap();

        let alice_session = r.get_session(&alice).unwrap();
        let bob_session = r.get_session(&bob).unwrap();
        assert_eq!(alice_session.messages[0].content, "alice msg");
        assert_eq!(bob_session.messages[0].content, "bob msg");
        assert_eq!(r.session_count(), 2);
    }
}
