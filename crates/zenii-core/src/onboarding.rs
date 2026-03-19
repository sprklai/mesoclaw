use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::credential::CredentialStore;

/// Setup status returned by the onboarding check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    /// Whether the user still needs to complete onboarding.
    pub needs_setup: bool,
    /// Which required fields are still missing.
    pub missing: Vec<String>,
    /// System-detected IANA timezone (if available).
    pub detected_timezone: Option<String>,
    /// Whether at least one AI provider has an API key and available models.
    pub has_usable_model: bool,
}

/// Check the current setup status.
///
/// `needs_setup` is true when:
/// - `user_name` is not set, OR
/// - `user_location` is not set, OR
/// - no provider has an API key configured
///
/// Timezone is NOT required (auto-detected via `iana-time-zone`).
pub async fn check_setup_status<C: CredentialStore + ?Sized>(
    config: &AppConfig,
    credentials: &C,
    provider_ids: &[String],
) -> SetupStatus {
    let mut missing = Vec::new();

    if config.user_name.is_none() || config.user_name.as_deref() == Some("") {
        missing.push("user_name".to_string());
    }
    if config.user_location.is_none() || config.user_location.as_deref() == Some("") {
        missing.push("user_location".to_string());
    }

    // Check if any provider has an API key
    let has_usable_model = has_any_api_key(credentials, provider_ids).await;
    if !has_usable_model {
        missing.push("api_key".to_string());
    }

    let detected_timezone = crate::ai::context::detect_system_timezone();

    SetupStatus {
        needs_setup: !missing.is_empty(),
        missing,
        detected_timezone,
        has_usable_model,
    }
}

/// Check if any of the given provider IDs has a stored API key.
async fn has_any_api_key<C: CredentialStore + ?Sized>(
    credentials: &C,
    provider_ids: &[String],
) -> bool {
    for id in provider_ids {
        let key = format!("api_key:{id}");
        match credentials.get(&key).await {
            Ok(Some(v)) if !v.is_empty() => return true,
            Ok(_) => {} // None or empty — no key stored
            Err(e) => {
                tracing::warn!(
                    "Credential access error for {key}: {e} \
                     (on macOS, this may indicate keychain access was revoked after binary recompilation)"
                );
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::credential::InMemoryCredentialStore;

    fn default_config() -> AppConfig {
        AppConfig::default()
    }

    fn config_with_profile() -> AppConfig {
        let mut cfg = AppConfig::default();
        cfg.user_name = Some("Alice".into());
        cfg.user_location = Some("Toronto, Canada".into());
        cfg
    }

    #[tokio::test]
    async fn needs_setup_when_no_name() {
        let cfg = default_config();
        let creds = InMemoryCredentialStore::new();
        let status = check_setup_status(&cfg, &creds, &["openai".into()]).await;
        assert!(status.needs_setup);
        assert!(status.missing.contains(&"user_name".to_string()));
    }

    #[tokio::test]
    async fn needs_setup_when_no_location() {
        let mut cfg = default_config();
        cfg.user_name = Some("Alice".into());
        let creds = InMemoryCredentialStore::new();
        let status = check_setup_status(&cfg, &creds, &["openai".into()]).await;
        assert!(status.needs_setup);
        assert!(status.missing.contains(&"user_location".to_string()));
    }

    #[tokio::test]
    async fn needs_setup_when_no_api_key() {
        let cfg = config_with_profile();
        let creds = InMemoryCredentialStore::new();
        let status = check_setup_status(&cfg, &creds, &["openai".into()]).await;
        assert!(status.needs_setup);
        assert!(status.missing.contains(&"api_key".to_string()));
    }

    #[tokio::test]
    async fn complete_when_all_set() {
        let cfg = config_with_profile();
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:openai", "sk-test").await.ok();
        let status = check_setup_status(&cfg, &creds, &["openai".into()]).await;
        assert!(!status.needs_setup);
        assert!(status.missing.is_empty());
        assert!(status.has_usable_model);
    }

    #[tokio::test]
    async fn detected_timezone_populated() {
        let cfg = default_config();
        let creds = InMemoryCredentialStore::new();
        let status = check_setup_status(&cfg, &creds, &[]).await;
        // Should have a detected timezone on any real system
        assert!(status.detected_timezone.is_some());
        // IANA timezones contain a '/'
        assert!(
            status
                .detected_timezone
                .as_ref()
                .is_some_and(|tz| tz.contains('/'))
        );
    }

    #[tokio::test]
    async fn empty_name_treated_as_missing() {
        let mut cfg = default_config();
        cfg.user_name = Some("".into());
        cfg.user_location = Some("Toronto".into());
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:openai", "sk-test").await.ok();
        let status = check_setup_status(&cfg, &creds, &["openai".into()]).await;
        assert!(status.needs_setup);
        assert!(status.missing.contains(&"user_name".to_string()));
    }

    #[tokio::test]
    async fn ollama_no_key_still_counts() {
        // Ollama doesn't require API key, but if it's the only provider and has no key,
        // has_usable_model is false. The UI/CLI should handle Ollama specially.
        let cfg = config_with_profile();
        let creds = InMemoryCredentialStore::new();
        let status = check_setup_status(&cfg, &creds, &["ollama".into()]).await;
        assert!(status.needs_setup);
        assert!(!status.has_usable_model);
    }
}
