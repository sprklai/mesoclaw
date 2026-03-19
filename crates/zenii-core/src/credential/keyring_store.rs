use async_trait::async_trait;

use crate::config::AppConfig;
use crate::{Result, ZeniiError};

use super::CredentialStore;

/// OS-native credential storage using the `keyring` crate.
///
/// Uses the OS keychain (macOS Keychain, Windows Credential Manager,
/// Linux Secret Service) for persistent, secure credential storage.
///
/// Maintains a key index in a special `_key_index` entry as a JSON array,
/// enabling the `list()` operation.
pub struct KeyringStore {
    service_id: String,
}

impl KeyringStore {
    const KEY_INDEX_ENTRY: &'static str = "_key_index";

    pub fn new(config: &AppConfig) -> Result<Self> {
        let service_id = config.keyring_service_id.clone();

        // Verify keyring access by doing a real set+get+delete probe
        let probe_key = "_probe";
        let probe_val = "zenii_probe";

        let entry = keyring::Entry::new(&service_id, probe_key)
            .map_err(|e| ZeniiError::Credential(format!("keyring unavailable: {e}")))?;

        entry
            .set_password(probe_val)
            .map_err(|e| ZeniiError::Credential(format!("keyring write probe failed: {e}")))?;

        let read = entry
            .get_password()
            .map_err(|e| ZeniiError::Credential(format!("keyring read probe failed: {e}")))?;

        if read != probe_val {
            return Err(ZeniiError::Credential(
                "keyring probe read-back mismatch".into(),
            ));
        }

        let _ = entry.delete_credential(); // cleanup, ignore errors

        Ok(Self { service_id })
    }

    /// Build with a specific service ID (for testing).
    pub fn with_service_id(service_id: &str) -> Result<Self> {
        let config = AppConfig {
            keyring_service_id: service_id.to_string(),
            ..Default::default()
        };
        Self::new(&config)
    }
}

#[async_trait]
impl CredentialStore for KeyringStore {
    async fn set(&self, key: &str, value: &str) -> Result<()> {
        let key = key.to_string();
        let value = value.to_string();
        let service_id = self.service_id.clone();

        // Clone self's data for the blocking task
        let store_service = service_id.clone();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&store_service, &key)
                .map_err(|e| ZeniiError::Credential(format!("keyring entry error: {e}")))?;

            entry
                .set_password(&value)
                .map_err(|e| ZeniiError::Credential(format!("keyring write error: {e}")))?;

            // Update key index
            let index_entry = keyring::Entry::new(&service_id, KeyringStore::KEY_INDEX_ENTRY)
                .map_err(|e| ZeniiError::Credential(format!("keyring entry error: {e}")))?;

            let mut keys: Vec<String> = match index_entry.get_password() {
                Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
                Err(keyring::Error::NoEntry) => vec![],
                Err(e) => {
                    return Err(ZeniiError::Credential(format!("keyring read error: {e}")));
                }
            };

            if !keys.contains(&key) {
                keys.push(key);
                keys.sort();
                let json = serde_json::to_string(&keys)
                    .map_err(|e| ZeniiError::Credential(format!("json error: {e}")))?;
                index_entry
                    .set_password(&json)
                    .map_err(|e| ZeniiError::Credential(format!("keyring write error: {e}")))?;
            }

            Ok(())
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }

    async fn get(&self, key: &str) -> Result<Option<String>> {
        let key = key.to_string();
        let service_id = self.service_id.clone();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service_id, &key)
                .map_err(|e| ZeniiError::Credential(format!("keyring entry error: {e}")))?;

            match entry.get_password() {
                Ok(val) => Ok(Some(val)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(ZeniiError::Credential(format!("keyring read error: {e}"))),
            }
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let key = key.to_string();
        let service_id = self.service_id.clone();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service_id, &key)
                .map_err(|e| ZeniiError::Credential(format!("keyring entry error: {e}")))?;

            let deleted = match entry.delete_credential() {
                Ok(()) => true,
                Err(keyring::Error::NoEntry) => false,
                Err(e) => {
                    return Err(ZeniiError::Credential(format!("keyring delete error: {e}")));
                }
            };

            if deleted {
                // Update key index
                let index_entry =
                    keyring::Entry::new(&service_id, KeyringStore::KEY_INDEX_ENTRY)
                        .map_err(|e| ZeniiError::Credential(format!("keyring entry error: {e}")))?;

                let mut keys: Vec<String> = match index_entry.get_password() {
                    Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
                    Err(_) => vec![],
                };

                keys.retain(|k| k != &key);
                let json = serde_json::to_string(&keys)
                    .map_err(|e| ZeniiError::Credential(format!("json error: {e}")))?;
                index_entry
                    .set_password(&json)
                    .map_err(|e| ZeniiError::Credential(format!("keyring write error: {e}")))?;
            }

            Ok(deleted)
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }

    async fn list(&self) -> Result<Vec<String>> {
        let service_id = self.service_id.clone();

        tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service_id, KeyringStore::KEY_INDEX_ENTRY)
                .map_err(|e| ZeniiError::Credential(format!("keyring entry error: {e}")))?;

            match entry.get_password() {
                Ok(json) => {
                    let mut keys: Vec<String> = serde_json::from_str(&json)
                        .map_err(|e| ZeniiError::Credential(format!("key index corrupt: {e}")))?;
                    keys.sort();
                    Ok(keys)
                }
                Err(keyring::Error::NoEntry) => Ok(vec![]),
                Err(e) => Err(ZeniiError::Credential(format!("keyring read error: {e}"))),
            }
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }
}

/// Try to create a KeyringStore, falling back to FileCredentialStore then InMemoryCredentialStore.
///
/// Fallback chain: KeyringStore (OS keyring) → FileCredentialStore (encrypted JSON) → InMemory (volatile).
/// Performs an async probe (set+get+delete via spawn_blocking) to verify the keyring
/// actually works end-to-end, since some Linux backends pass sync probes but fail async.
pub async fn keyring_or_fallback(config: &AppConfig) -> std::sync::Arc<dyn CredentialStore> {
    let store = match KeyringStore::new(config) {
        Ok(ks) => ks,
        Err(e) => {
            tracing::warn!("Keyring unavailable ({e}), trying file-based credential store");
            return file_or_in_memory_fallback(config);
        }
    };

    // Verify async round-trip works
    let probe_key = "_async_probe";
    let probe_val = "zenii_async_probe";
    if store.set(probe_key, probe_val).await.is_ok()
        && let Ok(Some(v)) = store.get(probe_key).await
    {
        let _ = store.delete(probe_key).await;
        if v == probe_val {
            tracing::info!("Using OS keyring for credential storage");
            return std::sync::Arc::new(store);
        }
    }

    tracing::warn!(
        "Keyring async probe failed, trying file-based credential store. \
         On macOS, this may occur after binary recompilation changes the code signature."
    );
    file_or_in_memory_fallback(config)
}

/// Try FileCredentialStore, then fall back to InMemoryCredentialStore.
fn file_or_in_memory_fallback(config: &AppConfig) -> std::sync::Arc<dyn CredentialStore> {
    use std::path::PathBuf;

    use super::file_store::FileCredentialStore;

    let result = if let Some(ref path) = config.credential_file_path {
        FileCredentialStore::with_path(PathBuf::from(path), &config.keyring_service_id)
    } else {
        let data_dir = config
            .data_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(crate::config::default_data_dir);
        FileCredentialStore::new(&data_dir, &config.keyring_service_id)
    };

    match result {
        Ok(store) => {
            tracing::info!(
                "Using encrypted file credential store at {}",
                store.path().display()
            );
            std::sync::Arc::new(store)
        }
        Err(e) => {
            tracing::warn!(
                "All persistent credential stores failed ({e}) — using in-memory (volatile)"
            );
            std::sync::Arc::new(super::InMemoryCredentialStore::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::CredentialStore;

    // KeyringStore tests require an actual OS keyring (Secret Service on Linux,
    // Keychain on macOS, Credential Manager on Windows). In CI without a
    // keyring daemon, these will fail gracefully.
    //
    // The fallback_to_in_memory test always works regardless.

    fn make_test_config() -> AppConfig {
        AppConfig {
            // Use a unique service ID to avoid collisions with real data
            keyring_service_id: format!("zenii-test-{}", uuid::Uuid::new_v4()),
            ..Default::default()
        }
    }

    /// Try to create and verify a KeyringStore works end-to-end via async ops.
    /// Returns None if keyring is unavailable or async ops don't work
    /// (e.g., Linux keyutils with thread-isolated keyrings).
    async fn try_create_store() -> Option<KeyringStore> {
        let config = make_test_config();
        let store = KeyringStore::new(&config).ok()?;

        // Verify async round-trip works (spawn_blocking may use different thread)
        if store.set("_async_probe", "probe_val").await.is_err() {
            return None;
        }
        match store.get("_async_probe").await {
            Ok(Some(v)) if v == "probe_val" => {
                let _ = store.delete("_async_probe").await;
                Some(store)
            }
            _ => None,
        }
    }

    #[tokio::test]
    async fn set_and_get_credential() {
        let Some(store) = try_create_store().await else {
            eprintln!("SKIP: keyring not available for async ops");
            return;
        };
        store.set("test-key", "test-value").await.unwrap();
        let val = store.get("test-key").await.unwrap();
        assert_eq!(val, Some("test-value".to_string()));

        // Cleanup
        let _ = store.delete("test-key").await;
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let Some(store) = try_create_store().await else {
            eprintln!("SKIP: keyring not available for async ops");
            return;
        };
        let val = store.get("nonexistent-key-xyz").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn delete_existing_credential() {
        let Some(store) = try_create_store().await else {
            eprintln!("SKIP: keyring not available for async ops");
            return;
        };
        store.set("del-key", "del-value").await.unwrap();
        let deleted = store.delete("del-key").await.unwrap();
        assert!(deleted);
        let val = store.get("del-key").await.unwrap();
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn delete_nonexistent_is_noop() {
        let Some(store) = try_create_store().await else {
            eprintln!("SKIP: keyring not available for async ops");
            return;
        };
        let deleted = store.delete("never-existed").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn list_returns_stored_keys() {
        let Some(store) = try_create_store().await else {
            eprintln!("SKIP: keyring not available for async ops");
            return;
        };
        store.set("key-b", "vb").await.unwrap();
        store.set("key-a", "va").await.unwrap();
        let keys = store.list().await.unwrap();
        assert!(keys.contains(&"key-a".to_string()));
        assert!(keys.contains(&"key-b".to_string()));
        // Should be sorted
        let pos_a = keys.iter().position(|k| k == "key-a").unwrap();
        let pos_b = keys.iter().position(|k| k == "key-b").unwrap();
        assert!(pos_a < pos_b);

        // Cleanup
        let _ = store.delete("key-a").await;
        let _ = store.delete("key-b").await;
    }

    #[tokio::test]
    async fn key_index_survives_set_delete() {
        let Some(store) = try_create_store().await else {
            eprintln!("SKIP: keyring not available for async ops");
            return;
        };
        store.set("surv-a", "v").await.unwrap();
        store.set("surv-b", "v").await.unwrap();
        store.delete("surv-a").await.unwrap();

        let keys = store.list().await.unwrap();
        assert!(!keys.contains(&"surv-a".to_string()));
        assert!(keys.contains(&"surv-b".to_string()));

        // Cleanup
        let _ = store.delete("surv-b").await;
    }

    #[tokio::test]
    async fn uses_configured_service_id() {
        let config = make_test_config();
        let service_id = config.keyring_service_id.clone();
        let store = match KeyringStore::new(&config) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("SKIP: keyring not available");
                return;
            }
        };
        assert_eq!(store.service_id, service_id);
    }

    #[tokio::test]
    async fn fallback_to_in_memory() {
        // Verify the fallback function always returns a usable store.
        // The returned store (keyring or in-memory) should support basic operations.
        let config = AppConfig::default();
        let store = keyring_or_fallback(&config).await;
        store.set("test", "val").await.unwrap();
        let val = store.get("test").await.unwrap();
        assert_eq!(val, Some("val".to_string()));
    }
}
