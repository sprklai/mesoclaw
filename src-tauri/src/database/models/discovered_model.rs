//! Discovered model database model.
//!
//! Stores dynamically discovered models from provider APIs with multi-modality support.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ai::providers::router::{CostTier, ModelCapabilities, ModelModality};
use crate::database::schema::discovered_models;
use crate::database::utils::{bool_to_int, int_to_bool};

/// Discovered model database model (Queryable)
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = discovered_models, check_for_backend(diesel::sqlite::Sqlite))]
pub struct DiscoveredModelRow {
    pub id: String,
    pub display_name: String,
    pub provider_id: String,
    pub model_id: String,
    pub cost_tier: String,
    pub context_limit: Option<i32>,
    pub modalities: String,
    pub capabilities: Option<String>,
    pub discovered_at: String,
    pub is_active: i32,
}

/// Discovered model data for frontend/API use
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredModelData {
    pub id: String,
    pub display_name: String,
    pub provider_id: String,
    pub model_id: String,
    pub cost_tier: String,
    pub context_limit: Option<i32>,
    pub modalities: Vec<String>,
    pub capabilities: Option<ModelCapabilities>,
    pub discovered_at: String,
    pub is_active: bool,
}

impl From<DiscoveredModelRow> for DiscoveredModelData {
    fn from(row: DiscoveredModelRow) -> Self {
        let modalities: Vec<String> = serde_json::from_str(&row.modalities).unwrap_or_else(|_| {
            log::warn!(
                "Failed to parse modalities for model {}, defaulting to text",
                row.id
            );
            vec!["text".to_string()]
        });

        let capabilities = row.capabilities.and_then(|s| serde_json::from_str(&s).ok());

        Self {
            id: row.id,
            display_name: row.display_name,
            provider_id: row.provider_id,
            model_id: row.model_id,
            cost_tier: row.cost_tier,
            context_limit: row.context_limit,
            modalities,
            capabilities,
            discovered_at: row.discovered_at,
            is_active: int_to_bool(row.is_active),
        }
    }
}

/// New discovered model for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = discovered_models)]
pub struct NewDiscoveredModel {
    pub id: String,
    pub display_name: String,
    pub provider_id: String,
    pub model_id: String,
    pub cost_tier: String,
    pub context_limit: Option<i32>,
    pub modalities: String,
    pub capabilities: Option<String>,
    pub is_active: i32,
}

impl NewDiscoveredModel {
    /// Create a new discovered model
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
        cost_tier: CostTier,
        context_limit: Option<i32>,
        modalities: Vec<ModelModality>,
        capabilities: Option<ModelCapabilities>,
    ) -> Self {
        let provider_id = provider_id.into();
        let model_id = model_id.into();

        // Create a unique ID from provider and model
        let id = format!("{}/{}", provider_id, model_id);

        // Serialize modalities to JSON
        let modalities_json = serde_json::to_string(
            &modalities
                .iter()
                .map(|m| modality_to_string(*m))
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[\"text\"]".to_string());

        // Serialize capabilities to JSON
        let capabilities_json = capabilities.and_then(|c| serde_json::to_string(&c).ok());

        Self {
            id,
            display_name: display_name.into(),
            provider_id,
            model_id,
            cost_tier: cost_tier_to_string(cost_tier),
            context_limit,
            modalities: modalities_json,
            capabilities: capabilities_json,
            is_active: 1,
        }
    }

    /// Create a text-only model
    pub fn text_only(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
        cost_tier: CostTier,
        context_limit: i32,
    ) -> Self {
        Self::new(
            provider_id,
            model_id,
            display_name,
            cost_tier,
            Some(context_limit),
            vec![ModelModality::Text],
            Some(ModelCapabilities::text_only()),
        )
    }

    /// Create a multimodal model (text + image)
    pub fn multimodal(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
        cost_tier: CostTier,
        context_limit: i32,
    ) -> Self {
        Self::new(
            provider_id,
            model_id,
            display_name,
            cost_tier,
            Some(context_limit),
            vec![ModelModality::Text, ModelModality::Image],
            Some(ModelCapabilities::full_featured()),
        )
    }

    /// Create from API discovery response
    pub fn from_api(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: Option<String>,
        cost_tier: CostTier,
        context_limit: Option<i32>,
        supports_vision: bool,
    ) -> Self {
        let provider_id = provider_id.into();
        let model_id = model_id.into();
        let display_name = display_name.unwrap_or_else(|| model_id.clone());

        let modalities = if supports_vision {
            vec![ModelModality::Text, ModelModality::Image]
        } else {
            vec![ModelModality::Text]
        };

        let capabilities = if supports_vision {
            ModelCapabilities::full_featured()
        } else {
            ModelCapabilities::text_only()
        };

        Self::new(
            provider_id,
            model_id,
            display_name,
            cost_tier,
            context_limit,
            modalities,
            Some(capabilities),
        )
    }
}

/// Discovered model update
///
/// Contains optional fields for partial updates.
#[derive(Debug, Clone)]
pub struct DiscoveredModelUpdate {
    pub display_name: Option<String>,
    pub cost_tier: Option<String>,
    pub context_limit: Option<i32>,
    pub modalities: Option<String>,
    pub capabilities: Option<String>,
    pub is_active: Option<i32>,
}

impl DiscoveredModelUpdate {
    /// Create an update to set active status
    pub fn set_active(active: bool) -> Self {
        Self {
            display_name: None,
            cost_tier: None,
            context_limit: None,
            modalities: None,
            capabilities: None,
            is_active: Some(bool_to_int(active)),
        }
    }

    /// Create an update for model capabilities
    pub fn update_capabilities(capabilities: ModelCapabilities) -> Self {
        Self {
            display_name: None,
            cost_tier: None,
            context_limit: None,
            modalities: None,
            capabilities: serde_json::to_string(&capabilities).ok(),
            is_active: None,
        }
    }
}

/// Helper: Convert CostTier to string for database storage
fn cost_tier_to_string(tier: CostTier) -> String {
    match tier {
        CostTier::Low => "low".to_string(),
        CostTier::Medium => "medium".to_string(),
        CostTier::High => "high".to_string(),
    }
}

/// Helper: Convert string to CostTier from database
pub fn string_to_cost_tier(s: &str) -> CostTier {
    match s.to_lowercase().as_str() {
        "low" => CostTier::Low,
        "high" => CostTier::High,
        _ => CostTier::Medium,
    }
}

/// Helper: Convert ModelModality to string for JSON serialization
fn modality_to_string(modality: ModelModality) -> String {
    match modality {
        ModelModality::Text => "text".to_string(),
        ModelModality::Image => "image".to_string(),
        ModelModality::ImageGeneration => "image_generation".to_string(),
        ModelModality::AudioTranscription => "audio_transcription".to_string(),
        ModelModality::AudioGeneration => "audio_generation".to_string(),
        ModelModality::Video => "video".to_string(),
        ModelModality::Embedding => "embedding".to_string(),
    }
}

/// Helper: Convert string to ModelModality
pub fn string_to_modality(s: &str) -> Option<ModelModality> {
    match s.to_lowercase().as_str() {
        "text" => Some(ModelModality::Text),
        "image" => Some(ModelModality::Image),
        "image_generation" => Some(ModelModality::ImageGeneration),
        "audio_transcription" => Some(ModelModality::AudioTranscription),
        "audio_generation" => Some(ModelModality::AudioGeneration),
        "video" => Some(ModelModality::Video),
        "embedding" => Some(ModelModality::Embedding),
        _ => None,
    }
}

/// Parse modalities from JSON string
pub fn parse_modalities(json: &str) -> Vec<ModelModality> {
    let strings: Vec<String> = serde_json::from_str(json).unwrap_or_default();
    strings
        .iter()
        .filter_map(|s| string_to_modality(s))
        .collect()
}

/// Model with provider info for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutableModel {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub cost_tier: String,
    pub modalities: Vec<String>,
    pub is_available: bool,
}

impl RoutableModel {
    /// Create from discovered model data
    pub fn from_data(data: &DiscoveredModelData) -> Self {
        Self {
            id: data.id.clone(),
            provider_id: data.provider_id.clone(),
            model_id: data.model_id.clone(),
            display_name: data.display_name.clone(),
            cost_tier: data.cost_tier.clone(),
            modalities: data.modalities.clone(),
            is_available: data.is_active,
        }
    }

    /// Check if this model supports all required modalities
    pub fn supports_modalities(&self, required: &[ModelModality]) -> bool {
        required.iter().all(|m| {
            let m_str = modality_to_string(*m);
            self.modalities.contains(&m_str)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_discovered_model_text_only() {
        let model = NewDiscoveredModel::text_only(
            "openai",
            "gpt-4o-mini",
            "GPT-4o Mini",
            CostTier::Low,
            128000,
        );

        assert_eq!(model.id, "openai/gpt-4o-mini");
        assert_eq!(model.display_name, "GPT-4o Mini");
        assert_eq!(model.cost_tier, "low");
        assert!(model.modalities.contains("text"));
    }

    #[test]
    fn test_new_discovered_model_multimodal() {
        let model = NewDiscoveredModel::multimodal(
            "anthropic",
            "claude-sonnet-4-5",
            "Claude Sonnet 4.5",
            CostTier::Medium,
            200000,
        );

        assert_eq!(model.id, "anthropic/claude-sonnet-4-5");
        assert_eq!(model.cost_tier, "medium");
        assert!(model.modalities.contains("text"));
        assert!(model.modalities.contains("image"));
    }

    #[test]
    fn test_cost_tier_conversion() {
        assert!(matches!(string_to_cost_tier("low"), CostTier::Low));
        assert!(matches!(string_to_cost_tier("medium"), CostTier::Medium));
        assert!(matches!(string_to_cost_tier("high"), CostTier::High));
        assert!(matches!(string_to_cost_tier("unknown"), CostTier::Medium));
    }

    #[test]
    fn test_modality_conversion() {
        assert!(matches!(
            string_to_modality("text"),
            Some(ModelModality::Text)
        ));
        assert!(matches!(
            string_to_modality("image"),
            Some(ModelModality::Image)
        ));
        assert!(matches!(
            string_to_modality("video"),
            Some(ModelModality::Video)
        ));
        assert!(string_to_modality("unknown").is_none());
    }

    #[test]
    fn test_parse_modalities() {
        let modalities = parse_modalities("[\"text\", \"image\"]");
        assert_eq!(modalities.len(), 2);
        assert!(modalities.contains(&ModelModality::Text));
        assert!(modalities.contains(&ModelModality::Image));
    }

    #[test]
    fn test_routable_model_supports_modalities() {
        let data = DiscoveredModelData {
            id: "test/model".to_string(),
            display_name: "Test Model".to_string(),
            provider_id: "test".to_string(),
            model_id: "model".to_string(),
            cost_tier: "medium".to_string(),
            context_limit: Some(128000),
            modalities: vec!["text".to_string(), "image".to_string()],
            capabilities: None,
            discovered_at: "2024-01-01T00:00:00Z".to_string(),
            is_active: true,
        };

        let routable = RoutableModel::from_data(&data);
        assert!(routable.supports_modalities(&[ModelModality::Text]));
        assert!(routable.supports_modalities(&[ModelModality::Text, ModelModality::Image]));
        assert!(!routable.supports_modalities(&[ModelModality::Video]));
    }
}
