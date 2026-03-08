use serde::{Deserialize, Serialize};

/// Discord-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub allowed_guild_ids: Vec<u64>,
    pub allowed_channel_ids: Vec<u64>,
}

impl DiscordConfig {
    /// Check if a guild is allowed (empty list = allow all).
    pub fn is_guild_allowed(&self, guild_id: u64) -> bool {
        self.allowed_guild_ids.is_empty() || self.allowed_guild_ids.contains(&guild_id)
    }

    /// Check if a channel is allowed (empty list = allow all).
    pub fn is_channel_allowed(&self, channel_id: u64) -> bool {
        self.allowed_channel_ids.is_empty() || self.allowed_channel_ids.contains(&channel_id)
    }

    pub fn from_app_config(_config: &crate::config::AppConfig) -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_app_config() {
        let config = crate::config::AppConfig::default();
        let dc = DiscordConfig::from_app_config(&config);
        assert!(dc.allowed_guild_ids.is_empty());
        assert!(dc.allowed_channel_ids.is_empty());
    }
}
