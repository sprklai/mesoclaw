use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::database::schema::{ai_models, ai_providers};
use crate::database::utils::{bool_to_int, int_to_bool};

/// AI Provider database model (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ai_providers)]
pub struct AIProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub requires_api_key: i32,
    pub is_active: i32,
    pub created_at: String,
    pub is_user_defined: i32,
}

/// Typed AI Provider with boolean conversions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIProviderData {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub requires_api_key: bool,
    pub is_active: bool,
    pub created_at: String,
    pub is_user_defined: bool,
}

impl From<AIProvider> for AIProviderData {
    fn from(provider: AIProvider) -> Self {
        Self {
            id: provider.id,
            name: provider.name,
            base_url: provider.base_url,
            requires_api_key: int_to_bool(provider.requires_api_key),
            is_active: int_to_bool(provider.is_active),
            created_at: provider.created_at,
            is_user_defined: int_to_bool(provider.is_user_defined),
        }
    }
}

/// New AI provider for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ai_providers)]
pub struct NewAIProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub requires_api_key: i32,
    pub is_active: i32,
    pub created_at: String,
    pub is_user_defined: i32,
}

impl NewAIProvider {
    /// Create a new AI provider for insertion
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        requires_api_key: bool,
    ) -> Self {
        Self::new_with_user_defined(id, name, base_url, requires_api_key, false)
    }

    /// Create a new user-defined AI provider for insertion
    pub fn user_defined(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        requires_api_key: bool,
    ) -> Self {
        Self::new_with_user_defined(id, name, base_url, requires_api_key, true)
    }

    /// Create a new AI provider with explicit is_user_defined flag
    fn new_with_user_defined(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        requires_api_key: bool,
        is_user_defined: bool,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            name: name.into(),
            base_url: base_url.into(),
            requires_api_key: bool_to_int(requires_api_key),
            is_active: 1, // Always active on creation
            created_at: now,
            is_user_defined: bool_to_int(is_user_defined),
        }
    }
}

/// AI Model database model (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ai_models)]
pub struct AIModel {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub context_limit: Option<i32>,
    pub is_custom: i32,
    pub is_active: i32,
    pub created_at: String,
}

/// Typed AI Model with boolean conversions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelData {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub context_limit: Option<i32>,
    pub is_custom: bool,
    pub is_active: bool,
    pub created_at: String,
}

impl From<AIModel> for AIModelData {
    fn from(model: AIModel) -> Self {
        Self {
            id: model.id,
            provider_id: model.provider_id,
            model_id: model.model_id,
            display_name: model.display_name,
            context_limit: model.context_limit,
            is_custom: int_to_bool(model.is_custom),
            is_active: int_to_bool(model.is_active),
            created_at: model.created_at,
        }
    }
}

/// New AI model for insertion (Insertable)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ai_models)]
pub struct NewAIModel {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub context_limit: Option<i32>,
    pub is_custom: i32,
    pub is_active: i32,
    pub created_at: String,
}

impl NewAIModel {
    /// Create a new AI model for insertion
    pub fn new(
        id: impl Into<String>,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
        context_limit: Option<i32>,
        is_custom: bool,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            display_name: display_name.into(),
            context_limit,
            is_custom: bool_to_int(is_custom),
            is_active: 1, // Always active on creation
            created_at: now,
        }
    }

    /// Create a new custom model (user-added)
    pub fn custom(
        id: impl Into<String>,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self::new(id, provider_id, model_id, display_name, None, true)
    }
}

/// Provider with its associated models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWithModels {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub requires_api_key: bool,
    pub is_active: bool,
    pub is_user_defined: bool,
    pub models: Vec<AIModelData>,
}

impl ProviderWithModels {
    /// Create from a provider and its models
    pub fn new(provider: AIProviderData, models: Vec<AIModelData>) -> Self {
        Self {
            id: provider.id,
            name: provider.name,
            base_url: provider.base_url,
            requires_api_key: provider.requires_api_key,
            is_active: provider.is_active,
            is_user_defined: provider.is_user_defined,
            models,
        }
    }
}

/// Provider with API key status for settings UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWithKeyStatus {
    #[serde(flatten)]
    pub provider: AIProviderData,
    pub has_api_key: bool,
}

impl ProviderWithKeyStatus {
    /// Create from a provider and API key status
    pub fn new(provider: AIProviderData, has_api_key: bool) -> Self {
        Self {
            provider,
            has_api_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_provider_creation() {
        let provider = NewAIProvider::new(
            "test-provider",
            "Test Provider",
            "https://api.test.com",
            true,
        );

        assert_eq!(provider.id, "test-provider");
        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.base_url, "https://api.test.com");
        assert_eq!(provider.requires_api_key, 1);
        assert_eq!(provider.is_active, 1);
        assert_eq!(provider.is_user_defined, 0);
    }

    #[test]
    fn test_user_defined_provider_creation() {
        let provider = NewAIProvider::user_defined(
            "custom-provider",
            "Custom Provider",
            "https://custom.api.com",
            true,
        );

        assert_eq!(provider.id, "custom-provider");
        assert_eq!(provider.name, "Custom Provider");
        assert_eq!(provider.base_url, "https://custom.api.com");
        assert_eq!(provider.requires_api_key, 1);
        assert_eq!(provider.is_active, 1);
        assert_eq!(provider.is_user_defined, 1);
    }

    #[test]
    fn test_new_model_creation() {
        let model = NewAIModel::new(
            "test-model",
            "test-provider",
            "test/model",
            "Test Model",
            Some(128000),
            false,
        );

        assert_eq!(model.id, "test-model");
        assert_eq!(model.provider_id, "test-provider");
        assert_eq!(model.model_id, "test/model");
        assert_eq!(model.display_name, "Test Model");
        assert_eq!(model.context_limit, Some(128000));
        assert_eq!(model.is_custom, 0);
        assert_eq!(model.is_active, 1);
    }

    #[test]
    fn test_custom_model_creation() {
        let model = NewAIModel::custom(
            "custom-model",
            "test-provider",
            "custom/model",
            "Custom Model",
        );

        assert_eq!(model.is_custom, 1);
        assert_eq!(model.context_limit, None);
    }

    #[test]
    fn test_provider_data_conversion() {
        let provider = AIProvider {
            id: "test".to_string(),
            name: "Test".to_string(),
            base_url: "https://test.com".to_string(),
            requires_api_key: 1,
            is_active: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_user_defined: 0,
        };

        let data = AIProviderData::from(provider);
        assert!(data.requires_api_key);
        assert!(data.is_active);
        assert!(!data.is_user_defined);
    }

    #[test]
    fn test_user_defined_provider_data_conversion() {
        let provider = AIProvider {
            id: "custom".to_string(),
            name: "Custom".to_string(),
            base_url: "https://custom.com".to_string(),
            requires_api_key: 1,
            is_active: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_user_defined: 1,
        };

        let data = AIProviderData::from(provider);
        assert!(data.is_user_defined);
    }

    #[test]
    fn test_model_data_conversion() {
        let model = AIModel {
            id: "test".to_string(),
            provider_id: "provider".to_string(),
            model_id: "model".to_string(),
            display_name: "Test Model".to_string(),
            context_limit: Some(128000),
            is_custom: 1,
            is_active: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let data = AIModelData::from(model);
        assert!(data.is_custom);
        assert!(data.is_active);
        assert_eq!(data.context_limit, Some(128000));
    }
}
