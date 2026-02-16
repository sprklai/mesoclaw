//! Skill settings persistence.
//!
//! Manages per-workspace skill configuration including enabled/disabled
//! state and per-skill overrides.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::database::DbPool;
use crate::database::schema::skill_settings;
use crate::skills::error::{SkillError, SkillResult};
use crate::skills::types::{SkillSettings, SkillUserConfig};

/// Service for managing skill settings.
pub struct SkillSettingsService {
    pool: DbPool,
}

/// Database record for skill settings (queryable).
#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = skill_settings)]
struct SkillSettingsRecord {
    pub workspace_id: String,
    pub settings_json: String,
    pub updated_at: String,
}

/// New skill settings for insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = skill_settings)]
struct NewSkillSettings<'a> {
    pub workspace_id: &'a str,
    pub settings_json: &'a str,
    pub updated_at: &'a str,
}

impl SkillSettingsService {
    /// Create a new skill settings service.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get skill settings for a workspace.
    ///
    /// Returns default settings if none are saved.
    pub fn get_settings(&self, workspace_id: &str) -> SkillResult<SkillSettings> {
        use crate::database::schema::skill_settings::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| SkillError::DatabaseError(e.to_string()))?;

        let result: Option<SkillSettingsRecord> = dsl::skill_settings
            .filter(dsl::workspace_id.eq(workspace_id))
            .first(&mut conn)
            .optional()
            .map_err(|e| SkillError::DatabaseError(e.to_string()))?;

        match result {
            Some(record) => {
                serde_json::from_str(&record.settings_json)
                    .map_err(|e| SkillError::DatabaseError(e.to_string()))
            }
            None => Ok(SkillSettings::default()),
        }
    }

    /// Save skill settings for a workspace.
    pub fn save_settings(&self, workspace_id: &str, settings: &SkillSettings) -> SkillResult<()> {
        use crate::database::schema::skill_settings::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| SkillError::DatabaseError(e.to_string()))?;

        let settings_json =
            serde_json::to_string(settings).map_err(|e| SkillError::DatabaseError(e.to_string()))?;

        let now = chrono::Utc::now().to_rfc3339();

        let new_settings = NewSkillSettings {
            workspace_id,
            settings_json: &settings_json,
            updated_at: &now,
        };

        // Upsert the settings
        diesel::replace_into(dsl::skill_settings)
            .values(&new_settings)
            .execute(&mut conn)
            .map_err(|e| SkillError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Enable a skill for a workspace.
    pub fn enable_skill(&self, workspace_id: &str, skill_id: &str) -> SkillResult<()> {
        let mut settings = self.get_settings(workspace_id)?;
        settings.enabled_skills.insert(skill_id.to_string());
        self.save_settings(workspace_id, &settings)
    }

    /// Disable a skill for a workspace.
    pub fn disable_skill(&self, workspace_id: &str, skill_id: &str) -> SkillResult<()> {
        let mut settings = self.get_settings(workspace_id)?;
        settings.enabled_skills.remove(skill_id);
        self.save_settings(workspace_id, &settings)
    }

    /// Set a skill's enabled state.
    pub fn set_skill_enabled(
        &self,
        workspace_id: &str,
        skill_id: &str,
        enabled: bool,
    ) -> SkillResult<()> {
        if enabled {
            self.enable_skill(workspace_id, skill_id)
        } else {
            self.disable_skill(workspace_id, skill_id)
        }
    }

    /// Get the enabled skills for a workspace.
    pub fn get_enabled_skills(&self, workspace_id: &str) -> SkillResult<HashSet<String>> {
        let settings = self.get_settings(workspace_id)?;
        Ok(settings.enabled_skills)
    }

    /// Update skill configuration.
    pub fn update_skill_config(
        &self,
        workspace_id: &str,
        skill_id: &str,
        config: SkillUserConfig,
    ) -> SkillResult<()> {
        let mut settings = self.get_settings(workspace_id)?;

        // Update enabled state based on config
        if config.enabled {
            settings.enabled_skills.insert(skill_id.to_string());
        } else {
            settings.enabled_skills.remove(skill_id);
        }

        settings
            .skill_configs
            .insert(skill_id.to_string(), config);
        self.save_settings(workspace_id, &settings)
    }

    /// Initialize settings for a workspace with default skills enabled.
    pub fn initialize_defaults(
        &self,
        workspace_id: &str,
        default_enabled: &[String],
    ) -> SkillResult<()> {
        let existing = self.get_settings(workspace_id)?;

        // Only initialize if no settings exist
        if existing.enabled_skills.is_empty() && existing.skill_configs.is_empty() {
            let settings = SkillSettings {
                enabled_skills: default_enabled.iter().cloned().collect(),
                skill_configs: HashMap::new(),
                auto_select: true,
            };
            self.save_settings(workspace_id, &settings)?;
        }

        Ok(())
    }

    /// Delete settings for a workspace.
    pub fn delete_settings(&self, workspace_id: &str) -> SkillResult<()> {
        use crate::database::schema::skill_settings::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| SkillError::DatabaseError(e.to_string()))?;

        diesel::delete(dsl::skill_settings.filter(dsl::workspace_id.eq(workspace_id)))
            .execute(&mut conn)
            .map_err(|e| SkillError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Toggle auto-select mode.
    pub fn set_auto_select(&self, workspace_id: &str, auto_select: bool) -> SkillResult<()> {
        let mut settings = self.get_settings(workspace_id)?;
        settings.auto_select = auto_select;
        self.save_settings(workspace_id, &settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_settings_default() {
        let settings = SkillSettings::default();
        assert!(settings.enabled_skills.is_empty());
        assert!(settings.skill_configs.is_empty());
        assert!(settings.auto_select);
    }

    #[test]
    fn test_skill_user_config_serialization() {
        let config = SkillUserConfig {
            enabled: true,
            priority_override: Some(150),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: SkillUserConfig = serde_json::from_str(&json).unwrap();

        assert!(parsed.enabled);
        assert_eq!(parsed.priority_override, Some(150));
    }
}
