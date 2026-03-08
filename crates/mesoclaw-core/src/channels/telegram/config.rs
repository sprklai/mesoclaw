use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

/// Controls who can DM the bot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DmPolicy {
    Allowlist,
    Open,
    Disabled,
}

impl Default for DmPolicy {
    fn default() -> Self {
        Self::Allowlist
    }
}

impl DmPolicy {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "open" => Self::Open,
            "disabled" => Self::Disabled,
            _ => Self::Allowlist,
        }
    }
}

/// Exponential backoff retry policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl RetryPolicy {
    /// Calculate delay for a given attempt using exponential backoff.
    /// delay = min(min_delay_ms * 2^attempt, max_delay_ms)
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let delay_ms = self
            .min_delay_ms
            .saturating_mul(2u64.saturating_pow(attempt));
        Duration::from_millis(delay_ms.min(self.max_delay_ms))
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            min_delay_ms: 1000,
            max_delay_ms: 60_000,
        }
    }
}

/// Telegram-specific configuration (non-secret tunables).
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub allowed_chat_ids: Vec<i64>,
    pub polling_timeout_secs: u32,
    pub dm_policy: DmPolicy,
    pub retry: RetryPolicy,
    pub require_group_mention: bool,
    pub bot_username: Option<String>,
}

impl TelegramConfig {
    pub fn from_app_config(config: &AppConfig) -> Self {
        Self {
            allowed_chat_ids: vec![],
            polling_timeout_secs: config.telegram_polling_timeout_secs,
            dm_policy: DmPolicy::from_str_lossy(&config.telegram_dm_policy),
            retry: RetryPolicy {
                min_delay_ms: config.telegram_retry_min_ms,
                max_delay_ms: config.telegram_retry_max_ms,
            },
            require_group_mention: config.telegram_require_group_mention,
            bot_username: None,
        }
    }
}

/// Recognized bot commands.
#[derive(Debug, Clone, PartialEq)]
pub enum BotCommand {
    Start,
    Status,
    Cancel,
    Help,
    Allow(i64),
    Models,
    Stats,
    Unknown(String),
}

/// Parse a bot command from message text. Returns None if not a command.
pub fn parse_bot_command(text: &str) -> Option<BotCommand> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    // Strip leading '/' and isolate the command word (before space or @)
    let without_slash = &trimmed[1..];
    let cmd_word = without_slash
        .split(|c: char| c.is_whitespace() || c == '@')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match cmd_word.as_str() {
        "start" => Some(BotCommand::Start),
        "status" => Some(BotCommand::Status),
        "cancel" => Some(BotCommand::Cancel),
        "help" => Some(BotCommand::Help),
        "models" => Some(BotCommand::Models),
        "stats" => Some(BotCommand::Stats),
        "allow" => {
            // Extract numeric argument
            let rest = without_slash
                .strip_prefix("allow")
                .unwrap_or("")
                .trim_start_matches(|c: char| c == '@' || c.is_alphabetic())
                .trim();
            if let Ok(id) = rest.parse::<i64>() {
                Some(BotCommand::Allow(id))
            } else {
                Some(BotCommand::Unknown("allow".into()))
            }
        }
        other => Some(BotCommand::Unknown(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dm_policy_default() {
        assert_eq!(DmPolicy::default(), DmPolicy::Allowlist);
    }

    #[test]
    fn dm_policy_from_str() {
        assert_eq!(DmPolicy::from_str_lossy("open"), DmPolicy::Open);
        assert_eq!(DmPolicy::from_str_lossy("Open"), DmPolicy::Open);
        assert_eq!(DmPolicy::from_str_lossy("OPEN"), DmPolicy::Open);
        assert_eq!(DmPolicy::from_str_lossy("disabled"), DmPolicy::Disabled);
        assert_eq!(DmPolicy::from_str_lossy("Disabled"), DmPolicy::Disabled);
        assert_eq!(DmPolicy::from_str_lossy("allowlist"), DmPolicy::Allowlist);
        assert_eq!(DmPolicy::from_str_lossy("anything"), DmPolicy::Allowlist);
    }

    #[test]
    fn retry_backoff_calc() {
        let policy = RetryPolicy {
            min_delay_ms: 1000,
            max_delay_ms: 60_000,
        };
        assert_eq!(policy.delay_for(0), Duration::from_millis(1000));
        assert_eq!(policy.delay_for(1), Duration::from_millis(2000));
        assert_eq!(policy.delay_for(2), Duration::from_millis(4000));
        assert_eq!(policy.delay_for(3), Duration::from_millis(8000));
    }

    #[test]
    fn retry_max_capped() {
        let policy = RetryPolicy {
            min_delay_ms: 1000,
            max_delay_ms: 60_000,
        };
        // 2^10 * 1000 = 1_024_000, capped to 60_000
        assert_eq!(policy.delay_for(10), Duration::from_millis(60_000));
        // 2^20 would overflow without saturation
        assert_eq!(policy.delay_for(20), Duration::from_millis(60_000));
    }

    #[test]
    fn config_from_app_config() {
        let app_config = AppConfig {
            telegram_polling_timeout_secs: 45,
            telegram_dm_policy: "open".into(),
            telegram_retry_min_ms: 500,
            telegram_retry_max_ms: 30_000,
            telegram_require_group_mention: false,
            ..Default::default()
        };

        let tg_config = TelegramConfig::from_app_config(&app_config);
        assert_eq!(tg_config.polling_timeout_secs, 45);
        assert_eq!(tg_config.dm_policy, DmPolicy::Open);
        assert_eq!(tg_config.retry.min_delay_ms, 500);
        assert_eq!(tg_config.retry.max_delay_ms, 30_000);
        assert!(!tg_config.require_group_mention);
    }
}
