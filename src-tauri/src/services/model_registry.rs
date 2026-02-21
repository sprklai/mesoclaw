//! Model Registry Service
//!
//! Manages discovered models from all providers, syncs with the database,
//! and provides fast in-memory access for routing decisions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ai::discovery::{
    DiscoveredModelInfo, GroqDiscovery, ModelDiscovery, OllamaDiscovery, OpenAIDiscovery,
    OpenRouterDiscovery, ProviderConfig, VercelGatewayDiscovery,
};
use crate::ai::providers::router::{CostTier, ModelModality, RoutingProfile};
use crate::database::DbPool;
use crate::database::models::{DiscoveredModelData, DiscoveredModelRow, RoutableModel};
use crate::database::schema::discovered_models;
use diesel::prelude::*;

/// Model Registry for managing discovered models
pub struct ModelRegistry {
    /// In-memory cache of discovered models keyed by provider_id/model_id
    cache: Arc<RwLock<HashMap<String, DiscoveredModelData>>>,
    /// Database connection pool
    pool: DbPool,
    /// Discovery implementations for each provider
    discoverers: HashMap<String, Box<dyn ModelDiscovery>>,
}

impl ModelRegistry {
    /// Create a new ModelRegistry with database pool
    pub fn new(pool: DbPool) -> Self {
        let mut discoverers: HashMap<String, Box<dyn ModelDiscovery>> = HashMap::new();
        discoverers.insert("ollama".to_string(), Box::new(OllamaDiscovery::new()));
        discoverers.insert("openai".to_string(), Box::new(OpenAIDiscovery::new()));
        discoverers.insert("groq".to_string(), Box::new(GroqDiscovery::new()));
        discoverers.insert(
            "vercel-ai-gateway".to_string(),
            Box::new(VercelGatewayDiscovery::new()),
        );
        discoverers.insert(
            "openrouter".to_string(),
            Box::new(OpenRouterDiscovery::new()),
        );

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            pool,
            discoverers,
        }
    }

    /// Load all models from database into cache
    pub async fn load_from_database(&self) -> Result<usize, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Database connection error: {}", e))?;

        let models = discovered_models::table
            .filter(discovered_models::is_active.eq(1))
            .select(DiscoveredModelRow::as_select())
            .load(&mut conn)
            .map_err(|e| format!("Failed to load models: {}", e))?;

        let mut cache = self.cache.write().await;
        cache.clear();

        for model in models {
            let data = DiscoveredModelData::from(model);
            cache.insert(data.id.clone(), data);
        }

        Ok(cache.len())
    }

    /// Discover models from a specific provider
    pub async fn discover_from_provider(
        &self,
        provider_id: &str,
        config: &ProviderConfig,
    ) -> Result<Vec<DiscoveredModelInfo>, String> {
        let discoverer = self
            .discoverers
            .get(provider_id)
            .ok_or_else(|| format!("No discoverer for provider: {}", provider_id))?;

        discoverer
            .discover_models(config)
            .await
            .map_err(|e| format!("Discovery failed: {}", e))
    }

    /// Discover and sync models from a specific provider
    pub async fn discover_and_sync(
        &self,
        provider_id: &str,
        config: &ProviderConfig,
    ) -> Result<usize, String> {
        let models = self.discover_from_provider(provider_id, config).await?;
        self.sync_models(provider_id, models).await
    }

    /// Sync discovered models to database
    pub async fn sync_models(
        &self,
        provider_id: &str,
        models: Vec<DiscoveredModelInfo>,
    ) -> Result<usize, String> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| format!("Database connection error: {}", e))?;

        let mut added_count = 0;

        // Deactivate all existing models for this provider
        diesel::update(discovered_models::table)
            .filter(discovered_models::provider_id.eq(provider_id))
            .set(discovered_models::is_active.eq(0))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to deactivate models: {}", e))?;

        for model_info in models {
            let new_model = model_info.to_new_discovered_model();

            // Try to insert, update if exists
            let result = diesel::insert_into(discovered_models::table)
                .values(&new_model)
                .on_conflict(discovered_models::id)
                .do_update()
                .set((
                    discovered_models::display_name.eq(&new_model.display_name),
                    discovered_models::cost_tier.eq(&new_model.cost_tier),
                    discovered_models::context_limit.eq(&new_model.context_limit),
                    discovered_models::modalities.eq(&new_model.modalities),
                    discovered_models::capabilities.eq(&new_model.capabilities),
                    discovered_models::is_active.eq(1),
                ))
                .execute(&mut conn);

            match result {
                Ok(_) => added_count += 1,
                Err(e) => {
                    log::warn!(
                        "Failed to sync model {}/{}: {}",
                        provider_id,
                        model_info.model_id,
                        e
                    );
                }
            }
        }

        // Reload cache
        self.load_from_database().await?;

        Ok(added_count)
    }

    /// Get all active models
    pub async fn get_all_models(&self) -> Vec<DiscoveredModelData> {
        let cache = self.cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get models for a specific provider
    pub async fn get_models_by_provider(&self, provider_id: &str) -> Vec<DiscoveredModelData> {
        let cache = self.cache.read().await;
        cache
            .values()
            .filter(|m| m.provider_id == provider_id)
            .cloned()
            .collect()
    }

    /// Get models matching a cost tier
    pub async fn get_models_by_cost_tier(&self, tier: CostTier) -> Vec<RoutableModel> {
        let tier_str = match tier {
            CostTier::Low => "low",
            CostTier::Medium => "medium",
            CostTier::High => "high",
        };

        let cache = self.cache.read().await;
        cache
            .values()
            .filter(|m| m.cost_tier == tier_str && m.is_active)
            .map(RoutableModel::from_data)
            .collect()
    }

    /// Get models matching a modality
    pub async fn get_models_by_modality(&self, modality: ModelModality) -> Vec<RoutableModel> {
        let modality_str = match modality {
            ModelModality::Text => "text",
            ModelModality::Image => "image",
            ModelModality::ImageGeneration => "image_generation",
            ModelModality::AudioTranscription => "audio_transcription",
            ModelModality::AudioGeneration => "audio_generation",
            ModelModality::Video => "video",
            ModelModality::Embedding => "embedding",
        };

        let cache = self.cache.read().await;
        cache
            .values()
            .filter(|m| m.is_active && m.modalities.iter().any(|mo| mo == modality_str))
            .map(RoutableModel::from_data)
            .collect()
    }

    /// Get best model for a profile
    pub async fn get_best_model_for_profile(
        &self,
        profile: RoutingProfile,
    ) -> Option<RoutableModel> {
        let preferred_tiers = match profile {
            RoutingProfile::Eco => &[CostTier::Low, CostTier::Medium, CostTier::High] as &[_],
            RoutingProfile::Balanced => &[CostTier::Medium, CostTier::Low, CostTier::High],
            RoutingProfile::Premium => &[CostTier::High, CostTier::Medium, CostTier::Low],
        };

        for tier in preferred_tiers {
            let models = self.get_models_by_cost_tier(*tier).await;
            if let Some(model) = models.first() {
                return Some(model.clone());
            }
        }

        None
    }

    /// Get best model for profile and modality
    pub async fn get_best_model_for_profile_and_modality(
        &self,
        profile: RoutingProfile,
        modalities: &[ModelModality],
    ) -> Option<RoutableModel> {
        let preferred_tiers = match profile {
            RoutingProfile::Eco => &[CostTier::Low, CostTier::Medium, CostTier::High] as &[_],
            RoutingProfile::Balanced => &[CostTier::Medium, CostTier::Low, CostTier::High],
            RoutingProfile::Premium => &[CostTier::High, CostTier::Medium, CostTier::Low],
        };

        let cache = self.cache.read().await;

        for tier in preferred_tiers {
            let tier_str = match tier {
                CostTier::Low => "low",
                CostTier::Medium => "medium",
                CostTier::High => "high",
            };

            let matching = cache
                .values()
                .filter(|m| {
                    m.is_active
                        && m.cost_tier == tier_str
                        && modalities.iter().all(|modality| {
                            let modality_str = match modality {
                                ModelModality::Text => "text",
                                ModelModality::Image => "image",
                                ModelModality::ImageGeneration => "image_generation",
                                ModelModality::AudioTranscription => "audio_transcription",
                                ModelModality::AudioGeneration => "audio_generation",
                                ModelModality::Video => "video",
                                ModelModality::Embedding => "embedding",
                            };
                            m.modalities.iter().any(|mo| mo == modality_str)
                        })
                })
                .map(RoutableModel::from_data)
                .collect::<Vec<_>>();

            if let Some(model) = matching.first() {
                return Some(model.clone());
            }
        }

        None
    }

    /// Get model by ID
    pub async fn get_model_by_id(&self, id: &str) -> Option<DiscoveredModelData> {
        let cache = self.cache.read().await;
        cache.get(id).cloned()
    }

    /// Get model by provider and model ID
    pub async fn get_model(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> Option<DiscoveredModelData> {
        let id = format!("{}/{}", provider_id, model_id);
        self.get_model_by_id(&id).await
    }

    /// Get count of cached models
    pub async fn model_count(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Check if a provider is available
    pub async fn is_provider_available(&self, provider_id: &str, config: &ProviderConfig) -> bool {
        if let Some(discoverer) = self.discoverers.get(provider_id) {
            discoverer.is_available(config).await
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    // Note: Integration tests would require a database connection
    // These tests focus on logic that doesn't require DB

    #[test]
    fn test_cost_tier_order() {
        use crate::ai::providers::router::CostTier;

        let eco_order: &[CostTier] = &[CostTier::Low, CostTier::Medium, CostTier::High];
        let balanced_order: &[CostTier] = &[CostTier::Medium, CostTier::Low, CostTier::High];
        let premium_order: &[CostTier] = &[CostTier::High, CostTier::Medium, CostTier::Low];

        assert_eq!(eco_order[0], CostTier::Low);
        assert_eq!(balanced_order[0], CostTier::Medium);
        assert_eq!(premium_order[0], CostTier::High);
    }
}
