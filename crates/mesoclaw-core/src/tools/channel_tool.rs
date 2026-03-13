use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::channels::message::ChannelMessage;
use crate::channels::registry::ChannelRegistry;
use crate::{MesoError, Result};

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
pub struct ChannelSendTool {
    registry: Arc<ChannelRegistry>,
}

impl ChannelSendTool {
    pub fn new(registry: Arc<ChannelRegistry>) -> Self {
        Self { registry }
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
        "Send messages to connected channels, list available channels, or check channel status. Your context includes available channels and known contacts. When sending, provide the recipient ID from your context. Actions: send, list, status."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["send", "list", "status"],
                    "description": "The channel operation to perform"
                },
                "channel": {
                    "type": "string",
                    "description": "Channel name (required for send/status)"
                },
                "message": {
                    "type": "string",
                    "description": "Message content to send (required for send)"
                },
                "recipient": {
                    "type": "string",
                    "description": "Recipient ID (e.g. chat_id for telegram, channel_id for slack/discord)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'action' field".into()))?;

        match action {
            "send" => {
                let channel = args["channel"]
                    .as_str()
                    .ok_or_else(|| MesoError::Validation("missing 'channel' for send".into()))?;
                let message = args["message"]
                    .as_str()
                    .ok_or_else(|| MesoError::Validation("missing 'message' for send".into()))?;

                let mut msg = ChannelMessage::new(channel, message).with_sender("agent");

                // If recipient provided, inject into metadata with channel-specific key
                if let Some(recipient) = args["recipient"].as_str() {
                    let meta_key = recipient_metadata_key(channel);
                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert(meta_key.to_string(), recipient.to_string());
                    msg = msg.with_metadata(metadata);
                }

                match self.registry.send(channel, msg).await {
                    Ok(()) => Ok(ToolResult::ok(format!("Message sent to '{channel}'"))),
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
                    .ok_or_else(|| MesoError::Validation("missing 'channel' for status".into()))?;

                match self.registry.status(channel) {
                    Some(status) => Ok(ToolResult::ok(format!("{channel}: {status:?}"))),
                    None => Ok(ToolResult::err(format!("Channel '{channel}' not found"))),
                }
            }
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: send, list, status"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::mpsc;

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

    fn setup_with_channel() -> ChannelSendTool {
        let registry = Arc::new(ChannelRegistry::new());
        registry
            .register(Arc::new(MockChannel::new("telegram")))
            .unwrap();
        ChannelSendTool::new(registry)
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
                "message": "hello"
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
        let tool = ChannelSendTool::new(registry);

        assert_eq!(tool.name(), "channel_send");
        assert!(tool.description().contains("channel"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
    }
}
