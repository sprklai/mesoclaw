pub mod file_store;
#[cfg(feature = "keyring")]
pub mod keyring_store;

use async_trait::async_trait;

use std::collections::HashMap;

use tokio::sync::Mutex;

use crate::Result;

/// Trait for storing and retrieving credentials (API keys, tokens, etc.)
#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn set(&self, key: &str, value: &str) -> Result<()>;
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn list(&self) -> Result<Vec<String>>;
}

/// In-memory credential store for testing and development.
pub struct InMemoryCredentialStore {
    store: Mutex<HashMap<String, String>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CredentialStore for InMemoryCredentialStore {
    async fn set(&self, key: &str, value: &str) -> Result<()> {
        self.store
            .lock()
            .await
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self.store.lock().await.get(key).cloned())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        Ok(self.store.lock().await.remove(key).is_some())
    }

    async fn list(&self) -> Result<Vec<String>> {
        let store = self.store.lock().await;
        let mut keys: Vec<String> = store.keys().cloned().collect();
        keys.sort();
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn set_and_get() {
        let store = InMemoryCredentialStore::new();
        store.set("api_key", "secret123").await.unwrap();
        assert_eq!(
            store.get("api_key").await.unwrap(),
            Some("secret123".to_string())
        );
    }

    #[tokio::test]
    async fn get_missing_key() {
        let store = InMemoryCredentialStore::new();
        assert_eq!(store.get("nonexistent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_existing_key() {
        let store = InMemoryCredentialStore::new();
        store.set("key", "val").await.unwrap();
        assert!(store.delete("key").await.unwrap());
        assert_eq!(store.get("key").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_missing_key() {
        let store = InMemoryCredentialStore::new();
        assert!(!store.delete("nope").await.unwrap());
    }

    #[tokio::test]
    async fn list_keys_sorted() {
        let store = InMemoryCredentialStore::new();
        store.set("zebra", "z").await.unwrap();
        store.set("alpha", "a").await.unwrap();
        store.set("middle", "m").await.unwrap();
        let keys = store.list().await.unwrap();
        assert_eq!(keys, vec!["alpha", "middle", "zebra"]);
    }

    #[tokio::test]
    async fn overwrite_value() {
        let store = InMemoryCredentialStore::new();
        store.set("key", "old").await.unwrap();
        store.set("key", "new").await.unwrap();
        assert_eq!(store.get("key").await.unwrap(), Some("new".to_string()));
    }

    #[tokio::test]
    async fn default_trait() {
        let store = InMemoryCredentialStore::default();
        assert!(store.list().await.unwrap().is_empty());
    }
}
