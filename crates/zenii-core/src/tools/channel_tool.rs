use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::ai::session::SessionManager;
use crate::channels::contacts;
use crate::channels::message::ChannelMessage;
use crate::channels::registry::ChannelRegistry;
use crate::channels::session_map::ChannelSessionMap;
use crate::db::DbPool;
use crate::event_bus::{AppEvent, EventBus};
use crate::{Result, ZeniiError};

use super::traits::{Tool, ToolResult};

/// Map channel name to the appropriate metadata key for recipient targeting.
fn recipient_metadata_key(channel: &str) -> &'static str {
    match channel {
        "telegram" => "chat_id",
        "slack" | "discord" => "channel_id",
        _ => "recipient_id",
    }
}

/// Agent tool for sending messages to channels and querying channel status.
/// Supports discovering known contacts and auto-resolving recipients.
pub struct ChannelSendTool {
    registry: Arc<ChannelRegistry>,
    db: DbPool,
    session_map: Arc<ChannelSessionMap>,
    session_manager: Arc<SessionManager>,
    event_bus: Arc<dyn EventBus>,
}

impl ChannelSendTool {
    pub fn new(
        registry: Arc<ChannelRegistry>,
        db: DbPool,
        session_map: Arc<ChannelSessionMap>,
        session_manager: Arc<SessionManager>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            registry,
            db,
            session_map,
            session_manager,
            event_bus,
        }
    }
}

#[async_trait]
impl Tool for ChannelSendTool {
    fn name(&self) -> &str {
        "channel_send"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Send messages to connected channels, list channels, check status, or discover known contacts. When sending without a recipient, auto-resolves if only one contact exists. Use 'contacts' action to list known recipients. Actions: send, list, status, contacts."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["send", "list", "status", "contacts"],
                    "description": "The channel operation to perform"
                },
                "channel": {
                    "type": "string",
                    "description": "Channel name (required for send/status, optional filter for contacts)"
                },
                "message": {
                    "type": "string",
                    "description": "Message content to send (required for send)"
                },
                "recipient": {
                    "type": "string",
                    "description": "Recipient ID (e.g. chat_id for telegram, channel_id for slack/discord). If omitted for send, auto-resolves when only one contact exists."
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("missing 'action' field".into()))?;

        match action {
            "send" => {
                let channel = args["channel"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'channel' for send".into()))?;
                let message = args["message"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'message' for send".into()))?;

                let mut msg = ChannelMessage::new(channel, message).with_sender("agent");

                // If recipient provided, inject into metadata with channel-specific key.
                // If not provided, try to auto-resolve from known contacts.
                if let Some(recipient) = args["recipient"].as_str() {
                    let meta_key = recipient_metadata_key(channel);
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert(meta_key.to_string(), recipient.to_string());
                    msg = msg.with_metadata(metadata);
                } else {
                    // Auto-resolve: query known contacts for this channel
                    let known = contacts::query_channel_contacts(&self.db, channel)
                        .await
                        .unwrap_or_default();
                    match known.len() {
                        0 => {
                            return Ok(ToolResult::err(format!(
                                "No known contacts for '{channel}'. \
                                 The recipient needs to message the bot first, \
                                 or provide a recipient ID explicitly."
                            )));
                        }
                        1 => {
                            let meta_key = recipient_metadata_key(channel);
                            let mut metadata = std::collections::HashMap::new();
                            metadata.insert(meta_key.to_string(), known[0].recipient_id.clone());
                            msg = msg.with_metadata(metadata);
                        }
                        _ => {
                            let listing = known
                                .iter()
                                .map(|c| format!("  - {} (id: {})", c.label, c.recipient_id))
                                .collect::<Vec<_>>()
                                .join("\n");
                            return Ok(ToolResult::err(format!(
                                "Multiple contacts found for '{channel}'. \
                                 Please specify a recipient:\n{listing}"
                            )));
                        }
                    }
                }

                match self.registry.send(channel, msg.clone()).await {
                    Ok(()) => {
                        // Persist outgoing message in channel session + notify frontend
                        let recipient_id =
                            msg.metadata.get(recipient_metadata_key(channel)).cloned();
                        if let Some(ref rid) = recipient_id {
                            let channel_key = format!("{channel}:{rid}");
                            if let Ok(sid) = self
                                .session_map
                                .resolve_session(&channel_key, channel)
                                .await
                            {
                                let _ = self
                                    .session_manager
                                    .append_message(&sid, "assistant", message)
                                    .await;
                                let _ = self.event_bus.publish(AppEvent::ChannelMessageReceived {
                                    channel: channel.to_string(),
                                    sender: "agent".to_string(),
                                    session_id: sid,
                                    content_preview: message.chars().take(100).collect(),
                                    role: "assistant".to_string(),
                                });
                            }
                        }
                        Ok(ToolResult::ok(format!("Message sent to '{channel}'")))
                    }
                    Err(e) => Ok(ToolResult::err(format!("Failed to send: {e}"))),
                }
            }
            "list" => {
                let names = self.registry.list();
                let channels: Vec<serde_json::Value> = names
                    .iter()
                    .map(|name| {
                        let status = self
                            .registry
                            .status(name)
                            .map(|s| format!("{s:?}"))
                            .unwrap_or_else(|| "unknown".to_string());
                        json!({ "name": name, "status": status })
                    })
                    .collect();
                Ok(ToolResult::ok(
                    serde_json::to_string_pretty(&channels).unwrap_or_default(),
                ))
            }
            "status" => {
                let channel = args["channel"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'channel' for status".into()))?;

                match self.registry.status(channel) {
                    Some(status) => Ok(ToolResult::ok(format!("{channel}: {status:?}"))),
                    None => Ok(ToolResult::err(format!("Channel '{channel}' not found"))),
                }
            }
            "contacts" => {
                let result = if let Some(channel) = args["channel"].as_str() {
                    contacts::query_channel_contacts(&self.db, channel).await?
                } else {
                    contacts::query_all_channel_contacts(&self.db).await?
                };
                let entries: Vec<serde_json::Value> = result
                    .iter()
                    .map(|c| {
                        json!({
                            "channel": c.channel,
                            "recipient_id": c.recipient_id,
                            "label": c.label
                        })
                    })
                    .collect();
                Ok(ToolResult::ok(
                    serde_json::to_string_pretty(&entries).unwrap_or_default(),
                ))
            }
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: send, list, status, contacts"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};
    use crate::db;
    use crate::event_bus::TokioBroadcastBus;
    use rusqlite::{Connection, params};
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::{Mutex, mpsc};

    struct MockChannel {
        name: String,
        sent: Arc<AtomicBool>,
    }

    impl MockChannel {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                sent: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    struct MockSender {
        channel_name: String,
        sent: Arc<AtomicBool>,
    }

    #[async_trait]
    impl ChannelSender for MockSender {
        fn channel_type(&self) -> &str {
            &self.channel_name
        }
        async fn send_message(&self, _message: ChannelMessage) -> crate::Result<()> {
            self.sent.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[async_trait]
    impl ChannelSender for MockChannel {
        fn channel_type(&self) -> &str {
            &self.name
        }
        async fn send_message(&self, _message: ChannelMessage) -> crate::Result<()> {
            self.sent.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[async_trait]
    impl ChannelLifecycle for MockChannel {
        fn display_name(&self) -> &str {
            &self.name
        }
        async fn connect(&self) -> crate::Result<()> {
            Ok(())
        }
        async fn disconnect(&self) -> crate::Result<()> {
            Ok(())
        }
        fn status(&self) -> ChannelStatus {
            ChannelStatus::Connected
        }
        fn create_sender(&self) -> Box<dyn ChannelSender> {
            Box::new(MockSender {
                channel_name: self.name.clone(),
                sent: self.sent.clone(),
            })
        }
    }

    #[async_trait]
    impl Channel for MockChannel {
        async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> crate::Result<()> {
            Ok(())
        }
        async fn health_check(&self) -> bool {
            true
        }
    }

    fn setup_db() -> DbPool {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT,
                channel_key TEXT,
                source TEXT NOT NULL DEFAULT 'api',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_channel_key ON sessions(channel_key);",
        )
        .expect("create tables");
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

    fn setup_with_channel() -> ChannelSendTool {
        let registry = Arc::new(ChannelRegistry::new());
        registry
            .register(Arc::new(MockChannel::new("telegram")))
            .unwrap();
        let db = setup_db();
        let session_manager = Arc::new(crate::ai::session::SessionManager::new(db.clone()));
        let session_map = Arc::new(ChannelSessionMap::new(session_manager.clone()));
        let event_bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        ChannelSendTool::new(registry, db, session_map, session_manager, event_bus)
    }

    // 17.15 — List channels returns registered channels
    #[tokio::test]
    async fn channel_tool_list() {
        let tool = setup_with_channel();
        let result = tool.execute(json!({ "action": "list" })).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("telegram"));
    }

    // 17.16 — Status returns channel status
    #[tokio::test]
    async fn channel_tool_status() {
        let tool = setup_with_channel();
        let result = tool
            .execute(json!({ "action": "status", "channel": "telegram" }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Connected"));
    }

    // 17.17 — Send to unknown channel returns error
    #[tokio::test]
    async fn channel_tool_send_unknown() {
        let tool = setup_with_channel();
        let result = tool
            .execute(json!({
                "action": "send",
                "channel": "nonexistent",
                "message": "hello",
                "recipient": "12345"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Failed to send"));
    }

    // 17.18 — Invalid action returns error
    #[tokio::test]
    async fn channel_tool_invalid_action() {
        let tool = setup_with_channel();
        let result = tool.execute(json!({ "action": "invalid" })).await.unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Unknown action"));
    }

    // 17.19 — Tool name/description/schema validation
    #[test]
    fn channel_tool_schema() {
        let registry = Arc::new(ChannelRegistry::new());
        let db = setup_db();
        let session_manager = Arc::new(crate::ai::session::SessionManager::new(db.clone()));
        let session_map = Arc::new(ChannelSessionMap::new(session_manager.clone()));
        let event_bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(16));
        let tool = ChannelSendTool::new(registry, db, session_map, session_manager, event_bus);

        assert_eq!(tool.name(), "channel_send");
        assert!(tool.description().contains("channel"));
        assert!(tool.description().contains("contacts"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
        let action_enum = schema["properties"]["action"]["enum"].as_array().unwrap();
        assert!(action_enum.contains(&json!("contacts")));
    }

    // Contacts action returns known contacts
    #[tokio::test]
    async fn contacts_action() {
        let tool = setup_with_channel();
        insert_session(&tool.db, "s1", Some("Mario Chat"), "telegram:12345").await;
        insert_session(&tool.db, "s2", Some("DevTeam"), "telegram:-98765").await;

        let result = tool
            .execute(json!({ "action": "contacts", "channel": "telegram" }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("12345"));
        assert!(result.output.contains("-98765"));
        assert!(result.output.contains("Mario Chat"));
    }

    // Send without recipient auto-resolves when single contact
    #[tokio::test]
    async fn send_auto_resolve_single() {
        let tool = setup_with_channel();
        insert_session(&tool.db, "s1", Some("Mario Chat"), "telegram:12345").await;

        let result = tool
            .execute(json!({
                "action": "send",
                "channel": "telegram",
                "message": "hello mario"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Message sent"));
    }

    // Send without recipient errors when no contacts
    #[tokio::test]
    async fn send_no_contacts_error() {
        let tool = setup_with_channel();

        let result = tool
            .execute(json!({
                "action": "send",
                "channel": "telegram",
                "message": "hello"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("No known contacts"));
    }

    // Send without recipient lists options when multiple contacts
    #[tokio::test]
    async fn send_multiple_contacts_lists() {
        let tool = setup_with_channel();
        insert_session(&tool.db, "s1", Some("Mario Chat"), "telegram:12345").await;
        insert_session(&tool.db, "s2", Some("DevTeam"), "telegram:-98765").await;

        let result = tool
            .execute(json!({
                "action": "send",
                "channel": "telegram",
                "message": "hello"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Multiple contacts"));
        assert!(result.output.contains("12345"));
        assert!(result.output.contains("-98765"));
    }
}
