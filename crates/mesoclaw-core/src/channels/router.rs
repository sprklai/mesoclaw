/// Channel-specific system context strings injected via preamble_override.
pub fn channel_system_context(channel_name: &str) -> &'static str {
    match channel_name {
        "telegram" => {
            "[Channel: Telegram] Keep responses concise and mobile-friendly. Avoid large code blocks. Use simple formatting."
        }
        "slack" => {
            "[Channel: Slack] Format using Slack mrkdwn. Use *bold* not **bold**. Keep responses professional."
        }
        "discord" => {
            "[Channel: Discord] Keep responses under 2000 characters. Use standard markdown."
        }
        _ => "[Channel: External] Keep responses concise.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CR.24 — channel_system_context returns telegram-specific prompt
    #[test]
    fn context_telegram() {
        let ctx = channel_system_context("telegram");
        assert!(ctx.contains("Telegram"));
        assert!(ctx.contains("mobile-friendly"));
    }

    // CR.25 — channel_system_context returns slack-specific prompt
    #[test]
    fn context_slack() {
        let ctx = channel_system_context("slack");
        assert!(ctx.contains("Slack"));
        assert!(ctx.contains("mrkdwn"));
    }

    // CR.26 — channel_system_context returns discord-specific prompt
    #[test]
    fn context_discord() {
        let ctx = channel_system_context("discord");
        assert!(ctx.contains("Discord"));
        assert!(ctx.contains("2000"));
    }

    // CR.27 — channel_system_context returns generic prompt for unknown channel
    #[test]
    fn context_unknown() {
        let ctx = channel_system_context("matrix");
        assert!(ctx.contains("External"));
        assert!(ctx.contains("concise"));
    }
}
