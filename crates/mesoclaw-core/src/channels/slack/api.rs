use serde_json::{Value, json};

/// Build JSON payload for Slack's chat.postMessage API.
pub fn post_message_payload(channel: &str, text: &str) -> Value {
    json!({
        "channel": channel,
        "text": text,
        "mrkdwn": true
    })
}

/// Build JSON payload for Slack's chat.update API.
pub fn update_message_payload(channel: &str, ts: &str, text: &str) -> Value {
    json!({
        "channel": channel,
        "ts": ts,
        "text": text,
        "mrkdwn": true
    })
}

/// Build the envelope acknowledgment for Socket Mode.
pub fn envelope_ack(envelope_id: &str) -> Value {
    json!({
        "envelope_id": envelope_id
    })
}

/// Check if a channel ID represents a DM (starts with 'D').
pub fn is_dm_channel(channel_id: &str) -> bool {
    channel_id.starts_with('D')
}

/// Check if text contains a bot mention (<@BOT_ID>).
pub fn contains_bot_mention(text: &str, bot_id: &str) -> bool {
    text.contains(&format!("<@{bot_id}>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_message_payload_test() {
        let payload = post_message_payload("C123", "hello world");
        assert_eq!(payload["channel"], "C123");
        assert_eq!(payload["text"], "hello world");
        assert_eq!(payload["mrkdwn"], true);
    }

    #[test]
    fn update_message_payload_test() {
        let payload = update_message_payload("C123", "1234567890.123456", "updated text");
        assert_eq!(payload["channel"], "C123");
        assert_eq!(payload["ts"], "1234567890.123456");
        assert_eq!(payload["text"], "updated text");
    }
}
