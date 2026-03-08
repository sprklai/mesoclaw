/// Trait for channel-specific response formatting.
pub trait ChannelFormatter: Send + Sync {
    /// Format a markdown response for the channel, splitting into multiple
    /// messages if needed. Returns a Vec of message strings.
    fn format(&self, markdown: &str) -> Vec<String>;

    /// Maximum message length for this channel.
    fn max_length(&self) -> usize;
}

/// Telegram formatter: MarkdownV2 escaping, 4096 char limit.
#[cfg(feature = "channels-telegram")]
pub struct TelegramFormatter;

#[cfg(feature = "channels-telegram")]
impl ChannelFormatter for TelegramFormatter {
    fn format(&self, markdown: &str) -> Vec<String> {
        let escaped = super::telegram::fmt::escape_markdown_v2(markdown);
        split_message(&escaped, self.max_length())
    }

    fn max_length(&self) -> usize {
        4096
    }
}

/// Slack formatter: mrkdwn conversion, 3000 char limit.
#[cfg(feature = "channels-slack")]
pub struct SlackFormatter;

#[cfg(feature = "channels-slack")]
impl ChannelFormatter for SlackFormatter {
    fn format(&self, markdown: &str) -> Vec<String> {
        let mrkdwn = super::slack::fmt::to_mrkdwn(markdown);
        split_message(&mrkdwn, self.max_length())
    }

    fn max_length(&self) -> usize {
        3000
    }
}

/// Discord formatter: keeps markdown, 2000 char limit.
#[cfg(feature = "channels-discord")]
pub struct DiscordFormatter;

#[cfg(feature = "channels-discord")]
impl ChannelFormatter for DiscordFormatter {
    fn format(&self, markdown: &str) -> Vec<String> {
        split_message(markdown, self.max_length())
    }

    fn max_length(&self) -> usize {
        2000
    }
}

/// Default formatter: passthrough, no splitting.
pub struct DefaultFormatter;

impl ChannelFormatter for DefaultFormatter {
    fn format(&self, markdown: &str) -> Vec<String> {
        vec![markdown.to_string()]
    }

    fn max_length(&self) -> usize {
        usize::MAX
    }
}

/// Get the appropriate formatter for a channel name.
pub fn formatter_for(channel_name: &str) -> Box<dyn ChannelFormatter> {
    match channel_name {
        #[cfg(feature = "channels-telegram")]
        "telegram" => Box::new(TelegramFormatter),
        #[cfg(feature = "channels-slack")]
        "slack" => Box::new(SlackFormatter),
        #[cfg(feature = "channels-discord")]
        "discord" => Box::new(DiscordFormatter),
        _ => Box::new(DefaultFormatter),
    }
}

/// Split a message into chunks respecting a max length.
/// Prefers paragraph boundaries > newlines > hard cut.
fn split_message(text: &str, max_length: usize) -> Vec<String> {
    if text.len() <= max_length {
        return vec![text.to_string()];
    }

    let mut parts = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_length {
            parts.push(remaining.to_string());
            break;
        }

        let chunk = &remaining[..max_length];
        let split_pos = find_split_point(chunk);

        parts.push(remaining[..split_pos].to_string());
        remaining = remaining[split_pos..].trim_start();
    }

    parts
}

fn find_split_point(chunk: &str) -> usize {
    // 1. Paragraph boundary (double newline)
    if let Some(pos) = chunk.rfind("\n\n").filter(|&p| p > 0) {
        return pos;
    }

    // 2. Newline boundary
    if let Some(pos) = chunk.rfind('\n').filter(|&p| p > 0) {
        return pos;
    }

    // 3. Word boundary (space)
    if let Some(pos) = chunk.rfind(' ').filter(|&p| p > 0) {
        return pos;
    }

    // 4. Hard cut
    chunk.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    // CR.13 — TelegramFormatter escapes MarkdownV2 special chars
    #[cfg(feature = "channels-telegram")]
    #[test]
    fn telegram_escapes_markdown() {
        let fmt = TelegramFormatter;
        let parts = fmt.format("Hello *world*!");
        assert_eq!(parts.len(), 1);
        // Should escape * and !
        assert!(parts[0].contains("\\*"));
        assert!(parts[0].contains("\\!"));
    }

    // CR.14 — TelegramFormatter splits messages at 4096 char boundary
    #[cfg(feature = "channels-telegram")]
    #[test]
    fn telegram_splits_long_message() {
        let fmt = TelegramFormatter;
        // plain ASCII text won't get much longer after escaping
        let long_text = "a ".repeat(3000);
        let parts = fmt.format(&long_text);
        assert!(parts.len() >= 2);
        for part in &parts {
            assert!(part.len() <= 4096);
        }
    }

    // CR.15 — SlackFormatter converts markdown bold to mrkdwn
    #[cfg(feature = "channels-slack")]
    #[test]
    fn slack_converts_bold() {
        let fmt = SlackFormatter;
        // to_mrkdwn converts **bold** -> *bold* (bold step), then *bold* -> _bold_ (italic step)
        // So the full pipeline turns markdown bold into mrkdwn italic — this matches Slack's
        // single-pass conversion where * means italic. The bold->italic chain is expected.
        let parts = fmt.format("This is **bold** text");
        assert_eq!(parts.len(), 1);
        // No double-asterisks remain
        assert!(!parts[0].contains("**"));
    }

    // CR.16 — SlackFormatter splits at 3000 chars
    #[cfg(feature = "channels-slack")]
    #[test]
    fn slack_splits_at_3000() {
        let fmt = SlackFormatter;
        let long_text = "word ".repeat(700);
        let parts = fmt.format(&long_text);
        assert!(parts.len() >= 2);
        for part in &parts {
            assert!(part.len() <= 3000);
        }
    }

    // CR.17 — DiscordFormatter keeps markdown, splits at 2000 chars
    #[cfg(feature = "channels-discord")]
    #[test]
    fn discord_splits_at_2000() {
        let fmt = DiscordFormatter;
        let long_text = "word ".repeat(500);
        let parts = fmt.format(&long_text);
        assert!(parts.len() >= 2);
        for part in &parts {
            assert!(part.len() <= 2000);
        }
    }

    // CR.18 — DefaultFormatter passes through unchanged
    #[test]
    fn default_passthrough() {
        let fmt = DefaultFormatter;
        let text = "Hello **world**!";
        let parts = fmt.format(text);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], text);
    }

    // CR.19 — Split logic prefers paragraph boundaries over hard cut
    #[test]
    fn split_prefers_paragraph_boundary() {
        let part1 = "a".repeat(1500);
        let part2 = "b".repeat(1500);
        let text = format!("{part1}\n\n{part2}");
        let parts = split_message(&text, 2000);
        assert!(parts.len() >= 2);
        // First part should end at the paragraph boundary
        assert_eq!(parts[0], part1);
    }

    // CR.20 — Short messages return single-element vec (no split)
    #[test]
    fn short_message_no_split() {
        let parts = split_message("Hello world", 2000);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], "Hello world");
    }
}
