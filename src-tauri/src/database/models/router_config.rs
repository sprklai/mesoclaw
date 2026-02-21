//! Router configuration database model.
//!
//! Stores the active routing profile and custom routing rules.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai::providers::router::{RoutingProfile, TaskType};
use crate::database::schema::router_config;

/// Router configuration database model (Queryable)
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = router_config, check_for_backend(diesel::sqlite::Sqlite))]
pub struct RouterConfigRow {
    pub id: i32,
    pub active_profile: String,
    pub custom_routes: Option<String>,
    pub task_overrides: Option<String>,
    pub last_discovery: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Router configuration data for frontend/API use
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterConfigData {
    pub active_profile: String,
    pub custom_routes: Option<serde_json::Value>,
    pub task_overrides: Option<serde_json::Value>,
    pub last_discovery: Option<String>,
}

impl From<RouterConfigRow> for RouterConfigData {
    fn from(row: RouterConfigRow) -> Self {
        Self {
            active_profile: row.active_profile,
            custom_routes: row
                .custom_routes
                .and_then(|s| serde_json::from_str(&s).ok()),
            task_overrides: row
                .task_overrides
                .and_then(|s| serde_json::from_str(&s).ok()),
            last_discovery: row.last_discovery,
        }
    }
}

/// New router configuration for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = router_config)]
pub struct NewRouterConfig {
    pub id: i32,
    pub active_profile: String,
    pub custom_routes: Option<String>,
    pub task_overrides: Option<String>,
    pub last_discovery: Option<String>,
}

impl NewRouterConfig {
    /// Create a new router configuration with default settings
    pub fn new() -> Self {
        Self {
            id: 1,
            active_profile: "balanced".to_string(),
            custom_routes: None,
            task_overrides: None,
            last_discovery: None,
        }
    }

    /// Create with a specific profile
    pub fn with_profile(profile: RoutingProfile) -> Self {
        Self {
            id: 1,
            active_profile: match profile {
                RoutingProfile::Eco => "eco",
                RoutingProfile::Balanced => "balanced",
                RoutingProfile::Premium => "premium",
            }
            .to_string(),
            custom_routes: None,
            task_overrides: None,
            last_discovery: None,
        }
    }
}

impl Default for NewRouterConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Router configuration update
///
/// Contains optional fields for partial updates.
#[derive(Debug, Clone)]
pub struct RouterConfigUpdate {
    pub active_profile: Option<String>,
    pub custom_routes: Option<String>,
    pub task_overrides: Option<String>,
    pub last_discovery: Option<String>,
}

impl RouterConfigUpdate {
    /// Create an update for the active profile
    pub fn set_profile(profile: RoutingProfile) -> Self {
        Self {
            active_profile: Some(
                match profile {
                    RoutingProfile::Eco => "eco",
                    RoutingProfile::Balanced => "balanced",
                    RoutingProfile::Premium => "premium",
                }
                .to_string(),
            ),
            custom_routes: None,
            task_overrides: None,
            last_discovery: None,
        }
    }

    /// Create an update for task overrides
    pub fn set_task_overrides(overrides: HashMap<String, String>) -> Self {
        Self {
            active_profile: None,
            custom_routes: None,
            task_overrides: serde_json::to_string(&overrides).ok(),
            last_discovery: None,
        }
    }

    /// Create an update for last discovery timestamp
    pub fn set_last_discovery() -> Self {
        Self {
            active_profile: None,
            custom_routes: None,
            task_overrides: None,
            last_discovery: Some(chrono::Utc::now().to_rfc3339()),
        }
    }
}

/// Parsed task overrides from JSON
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskOverrides {
    /// Map from task type to model ID
    #[serde(flatten)]
    pub overrides: HashMap<String, String>,
}

impl TaskOverrides {
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Get override for a task type
    pub fn get_override(&self, task: TaskType) -> Option<&str> {
        let key = match task {
            TaskType::Code => "code",
            TaskType::General => "general",
            TaskType::Fast => "fast",
            TaskType::Creative => "creative",
            TaskType::Analysis => "analysis",
            TaskType::Other => "other",
        };
        self.overrides.get(key).map(String::as_str)
    }

    /// Set override for a task type
    pub fn set_override(&mut self, task: TaskType, model_id: String) {
        let key = match task {
            TaskType::Code => "code",
            TaskType::General => "general",
            TaskType::Fast => "fast",
            TaskType::Creative => "creative",
            TaskType::Analysis => "analysis",
            TaskType::Other => "other",
        };
        self.overrides.insert(key.to_string(), model_id);
    }

    /// Clear override for a task type
    pub fn clear_override(&mut self, task: TaskType) {
        let key = match task {
            TaskType::Code => "code",
            TaskType::General => "general",
            TaskType::Fast => "fast",
            TaskType::Creative => "creative",
            TaskType::Analysis => "analysis",
            TaskType::Other => "other",
        };
        self.overrides.remove(key);
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_router_config() {
        let config = NewRouterConfig::new();
        assert_eq!(config.id, 1);
        assert_eq!(config.active_profile, "balanced");
        assert!(config.custom_routes.is_none());
        assert!(config.task_overrides.is_none());
    }

    #[test]
    fn test_router_config_with_profile() {
        let config = NewRouterConfig::with_profile(RoutingProfile::Premium);
        assert_eq!(config.active_profile, "premium");

        let config = NewRouterConfig::with_profile(RoutingProfile::Eco);
        assert_eq!(config.active_profile, "eco");
    }

    #[test]
    fn test_router_config_update_profile() {
        let update = RouterConfigUpdate::set_profile(RoutingProfile::Eco);
        assert_eq!(update.active_profile, Some("eco".to_string()));
        assert!(update.custom_routes.is_none());
    }

    #[test]
    fn test_task_overrides() {
        let mut overrides = TaskOverrides::default();
        assert!(overrides.get_override(TaskType::Code).is_none());

        overrides.set_override(TaskType::Code, "claude-opus-4".to_string());
        assert_eq!(
            overrides.get_override(TaskType::Code),
            Some("claude-opus-4")
        );

        overrides.clear_override(TaskType::Code);
        assert!(overrides.get_override(TaskType::Code).is_none());
    }

    #[test]
    fn test_task_overrides_json() {
        let mut overrides = TaskOverrides::default();
        overrides.set_override(TaskType::Code, "gpt-4o".to_string());
        overrides.set_override(TaskType::Analysis, "claude-opus".to_string());

        let json = overrides.to_json().unwrap();
        assert!(json.contains("code"));
        assert!(json.contains("gpt-4o"));

        let parsed = TaskOverrides::from_json(&json).unwrap();
        assert_eq!(parsed.get_override(TaskType::Code), Some("gpt-4o"));
        assert_eq!(parsed.get_override(TaskType::Analysis), Some("claude-opus"));
    }
}
