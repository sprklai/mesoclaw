use serde::{Deserialize, Serialize};

use crate::credential::CredentialStore;
use crate::db::{self, DbPool};
use crate::{MesoError, Result};

/// Embedded built-in provider definitions.
const BUILTIN_PROVIDERS_JSON: &str = include_str!("providers.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub requires_api_key: bool,
    pub is_active: bool,
    pub is_user_defined: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub context_limit: Option<i64>,
    pub supports_tools: bool,
    pub is_custom: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderWithModels {
    #[serde(flatten)]
    pub provider: ProviderInfo,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderWithKeyStatus {
    #[serde(flatten)]
    pub provider: ProviderInfo,
    pub models: Vec<ModelInfo>,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub message: String,
    pub model: Option<String>,
}

// JSON schema for providers.json
#[derive(Deserialize)]
struct BuiltinProviders {
    providers: Vec<BuiltinProvider>,
}

#[derive(Deserialize)]
struct BuiltinProvider {
    id: String,
    name: String,
    base_url: String,
    requires_api_key: bool,
    models: Vec<BuiltinModel>,
}

#[derive(Deserialize)]
struct BuiltinModel {
    model_id: String,
    display_name: String,
    context_limit: Option<i64>,
    #[serde(default = "default_supports_tools")]
    supports_tools: bool,
}

fn default_supports_tools() -> bool {
    true
}

/// Multi-provider AI management backed by SQLite.
pub struct ProviderRegistry {
    db: DbPool,
}

impl ProviderRegistry {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    /// Seed built-in providers from the embedded JSON. Idempotent.
    pub async fn seed_builtin_providers(&self) -> Result<()> {
        let builtin: BuiltinProviders = serde_json::from_str(BUILTIN_PROVIDERS_JSON)
            .map_err(|e| MesoError::Config(format!("failed to parse providers.json: {e}")))?;

        db::with_db(&self.db, move |conn| {
            for p in &builtin.providers {
                // INSERT OR IGNORE so existing rows are not overwritten
                conn.execute(
                    "INSERT OR IGNORE INTO ai_providers (id, name, base_url, requires_api_key, is_user_defined)
                     VALUES (?1, ?2, ?3, ?4, 0)",
                    rusqlite::params![p.id, p.name, p.base_url, p.requires_api_key as i32],
                )?;

                for m in &p.models {
                    let composite_id = format!("{}:{}", p.id, m.model_id);
                    conn.execute(
                        "INSERT OR IGNORE INTO ai_models (id, provider_id, model_id, display_name, context_limit, supports_tools, is_custom)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                        rusqlite::params![composite_id, p.id, m.model_id, m.display_name, m.context_limit, m.supports_tools as i32],
                    )?;
                }
            }
            Ok(())
        })
        .await
    }

    /// List all providers with their models.
    pub async fn list_providers(&self) -> Result<Vec<ProviderWithModels>> {
        db::with_db(&self.db, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, base_url, requires_api_key, is_active, is_user_defined, created_at
                 FROM ai_providers ORDER BY name",
            )?;

            let providers: Vec<ProviderInfo> = stmt
                .query_map([], |row| {
                    Ok(ProviderInfo {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        base_url: row.get(2)?,
                        requires_api_key: row.get::<_, i32>(3)? != 0,
                        is_active: row.get::<_, i32>(4)? != 0,
                        is_user_defined: row.get::<_, i32>(5)? != 0,
                        created_at: row.get(6)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            let mut result = Vec::with_capacity(providers.len());
            for provider in providers {
                let models = load_models_for_provider(conn, &provider.id)?;
                result.push(ProviderWithModels { provider, models });
            }

            Ok(result)
        })
        .await
    }

    /// List providers with API key status.
    pub async fn list_providers_with_key_status(
        &self,
        creds: &dyn CredentialStore,
    ) -> Result<Vec<ProviderWithKeyStatus>> {
        let providers = self.list_providers().await?;
        let stored_keys = creds.list().await?;

        let result = providers
            .into_iter()
            .map(|pwm| {
                let key_name = format!("api_key:{}", pwm.provider.id);
                let has_api_key = !pwm.provider.requires_api_key || stored_keys.contains(&key_name);
                ProviderWithKeyStatus {
                    provider: pwm.provider,
                    models: pwm.models,
                    has_api_key,
                }
            })
            .collect();

        Ok(result)
    }

    /// Get a single provider by ID with models.
    pub async fn get_provider(&self, id: &str) -> Result<ProviderWithModels> {
        let id = id.to_string();
        db::with_db(&self.db, move |conn| {
            let provider = conn
                .query_row(
                    "SELECT id, name, base_url, requires_api_key, is_active, is_user_defined, created_at
                     FROM ai_providers WHERE id = ?1",
                    [&id],
                    |row| {
                        Ok(ProviderInfo {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            base_url: row.get(2)?,
                            requires_api_key: row.get::<_, i32>(3)? != 0,
                            is_active: row.get::<_, i32>(4)? != 0,
                            is_user_defined: row.get::<_, i32>(5)? != 0,
                            created_at: row.get(6)?,
                        })
                    },
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        MesoError::NotFound(format!("provider not found: {id}"))
                    }
                    other => MesoError::from(other),
                })?;

            let models = load_models_for_provider(conn, &id)?;
            Ok(ProviderWithModels { provider, models })
        })
        .await
    }

    /// Look up a single model's info by provider and model ID.
    /// Returns `None` if the model is not found in the registry.
    pub async fn get_model_info(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Option<ModelInfo>> {
        let composite_id = format!("{provider_id}:{model_id}");
        db::with_db(&self.db, move |conn| {
            match conn.query_row(
                "SELECT id, provider_id, model_id, display_name, context_limit, supports_tools, is_custom, is_active
                 FROM ai_models WHERE id = ?1",
                [&composite_id],
                |row| {
                    Ok(ModelInfo {
                        id: row.get(0)?,
                        provider_id: row.get(1)?,
                        model_id: row.get(2)?,
                        display_name: row.get(3)?,
                        context_limit: row.get(4)?,
                        supports_tools: row.get::<_, i32>(5)? != 0,
                        is_custom: row.get::<_, i32>(6)? != 0,
                        is_active: row.get::<_, i32>(7)? != 0,
                    })
                },
            ) {
                Ok(info) => Ok(Some(info)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(MesoError::from(e)),
            }
        })
        .await
    }

    /// Add a user-defined provider.
    pub async fn add_user_provider(
        &self,
        id: &str,
        name: &str,
        base_url: &str,
        requires_api_key: bool,
        models: &[(String, String, bool)], // (model_id, display_name, supports_tools)
    ) -> Result<()> {
        // Validate ID: alphanumeric + hyphens
        if id.is_empty() || !id.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(MesoError::Validation(
                "provider ID must be non-empty and contain only alphanumeric characters and hyphens"
                    .into(),
            ));
        }

        let id = id.to_string();
        let name = name.to_string();
        let base_url = base_url.to_string();
        let models: Vec<(String, String, bool)> = models.to_vec();

        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO ai_providers (id, name, base_url, requires_api_key, is_user_defined)
                 VALUES (?1, ?2, ?3, ?4, 1)",
                rusqlite::params![id, name, base_url, requires_api_key as i32],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(ref err, _)
                    if err.code == rusqlite::ffi::ErrorCode::ConstraintViolation =>
                {
                    MesoError::Validation(format!("provider already exists: {id}"))
                }
                other => MesoError::from(other),
            })?;

            for (model_id, display_name, supports_tools) in &models {
                let composite_id = format!("{id}:{model_id}");
                conn.execute(
                    "INSERT INTO ai_models (id, provider_id, model_id, display_name, supports_tools, is_custom)
                     VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                    rusqlite::params![composite_id, id, model_id, display_name, *supports_tools as i32],
                )?;
            }

            Ok(())
        })
        .await
    }

    /// Update a provider's base_url.
    pub async fn update_provider(&self, id: &str, base_url: &str) -> Result<()> {
        let id = id.to_string();
        let base_url = base_url.to_string();

        db::with_db(&self.db, move |conn| {
            let rows = conn.execute(
                "UPDATE ai_providers SET base_url = ?1 WHERE id = ?2",
                rusqlite::params![base_url, id],
            )?;
            if rows == 0 {
                return Err(MesoError::NotFound(format!("provider not found: {id}")));
            }
            Ok(())
        })
        .await
    }

    /// Delete a user-defined provider. Built-in providers cannot be deleted.
    pub async fn delete_user_provider(&self, id: &str) -> Result<()> {
        let id = id.to_string();

        db::with_db(&self.db, move |conn| {
            // Check if provider is user-defined
            let is_user_defined: bool = conn
                .query_row(
                    "SELECT is_user_defined FROM ai_providers WHERE id = ?1",
                    [&id],
                    |row| row.get::<_, i32>(0).map(|v| v != 0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        MesoError::NotFound(format!("provider not found: {id}"))
                    }
                    other => MesoError::from(other),
                })?;

            if !is_user_defined {
                return Err(MesoError::Validation(format!(
                    "cannot delete built-in provider: {id}"
                )));
            }

            // Delete models first (foreign key), then provider
            conn.execute("DELETE FROM ai_models WHERE provider_id = ?1", [&id])?;
            conn.execute("DELETE FROM ai_providers WHERE id = ?1", [&id])?;

            Ok(())
        })
        .await
    }

    /// Add a custom model to an existing provider.
    pub async fn add_custom_model(
        &self,
        provider_id: &str,
        model_id: &str,
        display_name: &str,
        supports_tools: bool,
    ) -> Result<()> {
        let provider_id = provider_id.to_string();
        let model_id = model_id.to_string();
        let display_name = display_name.to_string();

        db::with_db(&self.db, move |conn| {
            // Verify provider exists
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) FROM ai_providers WHERE id = ?1",
                [&provider_id],
                |row| row.get::<_, i32>(0).map(|v| v > 0),
            )?;

            if !exists {
                return Err(MesoError::NotFound(format!(
                    "provider not found: {provider_id}"
                )));
            }

            let composite_id = format!("{provider_id}:{model_id}");
            conn.execute(
                "INSERT INTO ai_models (id, provider_id, model_id, display_name, supports_tools, is_custom)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                rusqlite::params![composite_id, provider_id, model_id, display_name, supports_tools as i32],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(ref err, _)
                    if err.code == rusqlite::ffi::ErrorCode::ConstraintViolation =>
                {
                    MesoError::Validation(format!("model already exists: {composite_id}"))
                }
                other => MesoError::from(other),
            })?;

            Ok(())
        })
        .await
    }

    /// Delete a custom model. Built-in models cannot be deleted.
    pub async fn delete_custom_model(&self, composite_id: &str) -> Result<()> {
        let composite_id = composite_id.to_string();

        db::with_db(&self.db, move |conn| {
            let is_custom: bool = conn
                .query_row(
                    "SELECT is_custom FROM ai_models WHERE id = ?1",
                    [&composite_id],
                    |row| row.get::<_, i32>(0).map(|v| v != 0),
                )
                .map_err(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => {
                        MesoError::NotFound(format!("model not found: {composite_id}"))
                    }
                    other => MesoError::from(other),
                })?;

            if !is_custom {
                return Err(MesoError::Validation(format!(
                    "cannot delete built-in model: {composite_id}"
                )));
            }

            conn.execute("DELETE FROM ai_models WHERE id = ?1", [&composite_id])?;
            Ok(())
        })
        .await
    }

    /// Get the global default model (stored in config table or a simple kv).
    /// Uses a simple row in ai_providers with a special marker.
    pub async fn get_default_model(&self) -> Result<Option<(String, String)>> {
        db::with_db(&self.db, |conn| {
            // We store the default in a simple approach: first active provider with a known model
            // Use a separate table-free approach via PRAGMA or a simple query
            // Actually, let's use a dedicated row approach
            match conn.query_row(
                "SELECT provider_id, model_id FROM ai_models
                 WHERE id = '_default_model'",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            ) {
                Ok(pair) => Ok(Some(pair)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(MesoError::from(e)),
            }
        })
        .await
    }

    /// Set the global default model.
    pub async fn set_default_model(&self, provider_id: &str, model_id: &str) -> Result<()> {
        let provider_id = provider_id.to_string();
        let model_id = model_id.to_string();

        db::with_db(&self.db, move |conn| {
            // Verify the model exists
            let composite_id = format!("{provider_id}:{model_id}");
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM ai_models WHERE id = ?1",
                    [&composite_id],
                    |row| row.get::<_, i32>(0).map(|v| v > 0),
                )?;

            if !exists {
                return Err(MesoError::NotFound(format!(
                    "model not found: {composite_id}"
                )));
            }

            // Upsert the default model marker
            conn.execute(
                "INSERT OR REPLACE INTO ai_models (id, provider_id, model_id, display_name, is_custom)
                 VALUES ('_default_model', ?1, ?2, 'Default', 0)",
                rusqlite::params![provider_id, model_id],
            )?;

            Ok(())
        })
        .await
    }
}

/// Load models for a specific provider.
fn load_models_for_provider(
    conn: &rusqlite::Connection,
    provider_id: &str,
) -> Result<Vec<ModelInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, provider_id, model_id, display_name, context_limit, supports_tools, is_custom, is_active
         FROM ai_models WHERE provider_id = ?1 AND id != '_default_model' ORDER BY display_name",
    )?;

    let models = stmt
        .query_map([provider_id], |row| {
            Ok(ModelInfo {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                model_id: row.get(2)?,
                display_name: row.get(3)?,
                context_limit: row.get(4)?,
                supports_tools: row.get::<_, i32>(5)? != 0,
                is_custom: row.get::<_, i32>(6)? != 0,
                is_active: row.get::<_, i32>(7)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::InMemoryCredentialStore;
    use tempfile::TempDir;

    async fn test_registry() -> (TempDir, ProviderRegistry) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&db_path).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();
        let registry = ProviderRegistry::new(pool);
        (dir, registry)
    }

    #[tokio::test]
    async fn seed_builtin_idempotent() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();
        registry.seed_builtin_providers().await.unwrap(); // second call should not error

        let providers = registry.list_providers().await.unwrap();
        assert!(providers.len() >= 6); // 6 built-in providers
    }

    #[tokio::test]
    async fn list_providers_returns_seeded() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        let providers = registry.list_providers().await.unwrap();
        let ids: Vec<&str> = providers.iter().map(|p| p.provider.id.as_str()).collect();
        assert!(ids.contains(&"openai"));
        assert!(ids.contains(&"anthropic"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"openrouter"));
        assert!(ids.contains(&"ollama"));
    }

    #[tokio::test]
    async fn get_provider_by_id() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        let p = registry.get_provider("openai").await.unwrap();
        assert_eq!(p.provider.name, "OpenAI");
        assert!(!p.models.is_empty());
    }

    #[tokio::test]
    async fn get_nonexistent_provider_errors() {
        let (_dir, registry) = test_registry().await;
        let result = registry.get_provider("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::NotFound(_)));
    }

    #[tokio::test]
    async fn add_user_provider() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        let models = vec![("my-model".to_string(), "My Model".to_string(), true)];
        registry
            .add_user_provider(
                "my-gateway",
                "My Gateway",
                "https://my.api/v1",
                true,
                &models,
            )
            .await
            .unwrap();

        let p = registry.get_provider("my-gateway").await.unwrap();
        assert_eq!(p.provider.name, "My Gateway");
        assert!(p.provider.is_user_defined);
        assert_eq!(p.models.len(), 1);
        assert_eq!(p.models[0].model_id, "my-model");
    }

    #[tokio::test]
    async fn add_user_provider_validates_id() {
        let (_dir, registry) = test_registry().await;

        let result = registry
            .add_user_provider("invalid id!", "Bad", "http://x", false, &[])
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::Validation(_)));

        let result = registry
            .add_user_provider("", "Empty", "http://x", false, &[])
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_user_provider() {
        let (_dir, registry) = test_registry().await;
        registry
            .add_user_provider("temp-prov", "Temp", "http://temp", false, &[])
            .await
            .unwrap();

        registry.delete_user_provider("temp-prov").await.unwrap();
        let result = registry.get_provider("temp-prov").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_builtin_provider_fails() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        let result = registry.delete_user_provider("openai").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::Validation(_)));
    }

    #[tokio::test]
    async fn update_provider_base_url() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        registry
            .update_provider("openai", "https://my-proxy.com/v1")
            .await
            .unwrap();

        let p = registry.get_provider("openai").await.unwrap();
        assert_eq!(p.provider.base_url, "https://my-proxy.com/v1");
    }

    #[tokio::test]
    async fn add_custom_model() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        registry
            .add_custom_model("openai", "ft:gpt-4o:my-org", "My Fine-tuned GPT-4o", true)
            .await
            .unwrap();

        let p = registry.get_provider("openai").await.unwrap();
        let custom = p.models.iter().find(|m| m.model_id == "ft:gpt-4o:my-org");
        assert!(custom.is_some());
        assert!(custom.unwrap().is_custom);
    }

    #[tokio::test]
    async fn delete_custom_model() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        registry
            .add_custom_model("openai", "custom-model", "Custom", true)
            .await
            .unwrap();

        registry
            .delete_custom_model("openai:custom-model")
            .await
            .unwrap();

        let p = registry.get_provider("openai").await.unwrap();
        assert!(!p.models.iter().any(|m| m.model_id == "custom-model"));
    }

    #[tokio::test]
    async fn delete_builtin_model_fails() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        let result = registry.delete_custom_model("openai:gpt-4o").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::Validation(_)));
    }

    #[tokio::test]
    async fn set_and_get_default_model() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        registry
            .set_default_model("openai", "gpt-4o")
            .await
            .unwrap();

        let default = registry.get_default_model().await.unwrap();
        assert_eq!(default, Some(("openai".into(), "gpt-4o".into())));
    }

    #[tokio::test]
    async fn get_default_model_none() {
        let (_dir, registry) = test_registry().await;
        let default = registry.get_default_model().await.unwrap();
        assert_eq!(default, None);
    }

    #[tokio::test]
    async fn list_providers_with_key_status() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:openai", "sk-test").await.unwrap();

        let providers = registry
            .list_providers_with_key_status(&creds)
            .await
            .unwrap();

        let openai = providers
            .iter()
            .find(|p| p.provider.id == "openai")
            .unwrap();
        assert!(openai.has_api_key);

        let anthropic = providers
            .iter()
            .find(|p| p.provider.id == "anthropic")
            .unwrap();
        assert!(!anthropic.has_api_key);
    }

    #[tokio::test]
    async fn ollama_no_api_key() {
        let (_dir, registry) = test_registry().await;
        registry.seed_builtin_providers().await.unwrap();

        let creds = InMemoryCredentialStore::new();
        let providers = registry
            .list_providers_with_key_status(&creds)
            .await
            .unwrap();

        let ollama = providers
            .iter()
            .find(|p| p.provider.id == "ollama")
            .unwrap();
        assert!(ollama.has_api_key); // Ollama doesn't require a key, so has_api_key is true
        assert!(!ollama.provider.requires_api_key);
    }
}
