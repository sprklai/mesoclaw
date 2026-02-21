//! Router Service
//!
//! Manages routing state, profiles, and coordinates model selection.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ai::providers::{CostTier, ModelModality, ModelRouter, RoutingProfile, TaskType};
use crate::database::DbPool;
use crate::database::models::{NewRouterConfig, RouterConfigData, RouterConfigRow, TaskOverrides};
use crate::database::schema::router_config;
use crate::services::model_registry::ModelRegistry;
use diesel::prelude::*;

/// Router Service for managing routing state
pub struct RouterService {
    /// Current active profile
    profile: Arc<RwLock<RoutingProfile>>,
    /// Task overrides (task type -> model ID)
    task_overrides: Arc<RwLock<TaskOverrides>>,
    /// Model registry for model selection
    registry: Arc<ModelRegistry>,
    /// Database pool for persistence
    pool: DbPool,
}

impl RouterService {
    /// Create a new RouterService
    pub fn new(pool: DbPool, registry: Arc<ModelRegistry>) -> Self {
        Self {
            profile: Arc::new(RwLock::new(RoutingProfile::default())),
            task_overrides: Arc::new(RwLock::new(TaskOverrides::default())),
            registry,
            pool,
        }
    }

    /// Initialize router state from database
    pub async fn load_from_database(&self) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Database connection error: {}", e))?;

        let config = router_config::table
            .filter(router_config::id.eq(1))
            .select(RouterConfigRow::as_select())
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to load router config: {}", e))?;

        if let Some(config) = config {
            // Set profile
            let profile = match config.active_profile.as_str() {
                "eco" => RoutingProfile::Eco,
                "premium" => RoutingProfile::Premium,
                _ => RoutingProfile::Balanced,
            };
            *self.profile.write().await = profile;

            // Load task overrides
            if let Some(overrides_json) = config.task_overrides {
                if let Some(overrides) = TaskOverrides::from_json(&overrides_json) {
                    *self.task_overrides.write().await = overrides;
                }
            }
        } else {
            // Create default config
            let new_config = NewRouterConfig::new();
            diesel::insert_into(router_config::table)
                .values(&new_config)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to create default router config: {}", e))?;
        }

        Ok(())
    }

    /// Get the current active profile
    pub async fn get_profile(&self) -> RoutingProfile {
        *self.profile.read().await
    }

    /// Set the active profile
    pub async fn set_profile(&self, profile: RoutingProfile) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Database connection error: {}", e))?;

        let profile_str = match profile {
            RoutingProfile::Eco => "eco",
            RoutingProfile::Balanced => "balanced",
            RoutingProfile::Premium => "premium",
        };

        diesel::update(router_config::table.filter(router_config::id.eq(1)))
            .set(router_config::active_profile.eq(profile_str))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to update profile: {}", e))?;

        *self.profile.write().await = profile;

        log::info!("[Router] Profile changed to: {:?}", profile);
        Ok(())
    }

    /// Get task overrides
    pub async fn get_task_overrides(&self) -> TaskOverrides {
        self.task_overrides.read().await.clone()
    }

    /// Set a task override
    pub async fn set_task_override(&self, task: TaskType, model_id: String) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Database connection error: {}", e))?;

        // Update in-memory
        self.task_overrides
            .write()
            .await
            .set_override(task, model_id.clone());

        // Serialize and save to database
        let overrides = self.task_overrides.read().await.clone();
        let overrides_json = overrides.to_json().unwrap_or_default();

        diesel::update(router_config::table.filter(router_config::id.eq(1)))
            .set(router_config::task_overrides.eq(Some(overrides_json)))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to update task overrides: {}", e))?;

        log::info!("[Router] Task override set: {:?} -> {}", task, model_id);
        Ok(())
    }

    /// Clear a task override
    pub async fn clear_task_override(&self, task: TaskType) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Database connection error: {}", e))?;

        // Update in-memory
        self.task_overrides.write().await.clear_override(task);

        // Serialize and save to database
        let overrides = self.task_overrides.read().await.clone();
        let overrides_json = overrides.to_json();

        diesel::update(router_config::table.filter(router_config::id.eq(1)))
            .set(router_config::task_overrides.eq(overrides_json))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to clear task override: {}", e))?;

        log::info!("[Router] Task override cleared: {:?}", task);
        Ok(())
    }

    /// Route a message to the appropriate model
    /// Uses task classification heuristics if no explicit override is set
    pub async fn route(&self, message: &str) -> Option<String> {
        // First check for task override
        let task = ModelRouter::classify_task(message);
        let task_str = match task {
            TaskType::Code => "code",
            TaskType::General => "general",
            TaskType::Fast => "fast",
            TaskType::Creative => "creative",
            TaskType::Analysis => "analysis",
            TaskType::Other => "other",
        };

        // Check for override
        if let Some(override_model) = self.task_overrides.read().await.get_override(task) {
            log::info!(
                "[Router] Using task override: {:?} -> {}",
                task,
                override_model
            );
            return Some(override_model.to_string());
        }

        // Use profile-based routing
        let profile = *self.profile.read().await;
        if let Some(model) = self.registry.get_best_model_for_profile(profile).await {
            log::info!(
                "[Router] Routed to model {} (profile: {:?}, task: {:?})",
                model.model_id,
                profile,
                task_str
            );
            return Some(model.model_id);
        }

        log::warn!("[Router] No suitable model found for message");
        None
    }

    /// Route a message to the appropriate model, considering modalities
    pub async fn route_with_modalities(
        &self,
        message: &str,
        modalities: &[ModelModality],
    ) -> Option<String> {
        let task = ModelRouter::classify_task(message);

        // Check for override
        if let Some(override_model) = self.task_overrides.read().await.get_override(task) {
            return Some(override_model.to_string());
        }

        let profile = *self.profile.read().await;
        if let Some(model) = self
            .registry
            .get_best_model_for_profile_and_modality(profile, modalities)
            .await
        {
            return Some(model.model_id);
        }

        // Fallback to simple routing
        self.route(message).await
    }

    /// Get all available models for the current profile
    pub async fn get_available_models(&self) -> Vec<String> {
        let profile = *self.profile.read().await;
        let tier = match profile {
            RoutingProfile::Eco => CostTier::Low,
            RoutingProfile::Balanced => CostTier::Medium,
            RoutingProfile::Premium => CostTier::High,
        };

        self.registry
            .get_models_by_cost_tier(tier)
            .await
            .into_iter()
            .map(|m| m.model_id)
            .collect()
    }

    /// Get the router configuration for frontend
    pub async fn get_config(&self) -> RouterConfigData {
        let profile = *self.profile.read().await;
        let overrides = self.task_overrides.read().await.clone();

        let profile_str = match profile {
            RoutingProfile::Eco => "eco",
            RoutingProfile::Balanced => "balanced",
            RoutingProfile::Premium => "premium",
        };

        let overrides_json = overrides.to_json();
        let overrides_value = overrides_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());

        RouterConfigData {
            active_profile: profile_str.to_string(),
            custom_routes: None,
            task_overrides: overrides_value,
            last_discovery: None,
        }
    }

    /// Record last discovery timestamp
    pub async fn record_discovery(&self) -> Result<(), String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Database connection error: {}", e))?;

        let now = chrono::Utc::now().to_rfc3339();

        diesel::update(router_config::table.filter(router_config::id.eq(1)))
            .set(router_config::last_discovery.eq(Some(now.clone())))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to record discovery: {}", e))?;

        log::info!("[Router] Last discovery recorded: {}", now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ai::providers::router::{RoutingProfile, TaskType};

    #[test]
    fn test_profile_string_conversion() {
        let profile_str = match RoutingProfile::Eco {
            RoutingProfile::Eco => "eco",
            RoutingProfile::Balanced => "balanced",
            RoutingProfile::Premium => "premium",
        };
        assert_eq!(profile_str, "eco");
    }

    #[test]
    fn test_task_string_conversion() {
        let task_str = match TaskType::Code {
            TaskType::Code => "code",
            TaskType::General => "general",
            TaskType::Fast => "fast",
            TaskType::Creative => "creative",
            TaskType::Analysis => "analysis",
            TaskType::Other => "other",
        };
        assert_eq!(task_str, "code");
    }
}
