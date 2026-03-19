use std::collections::HashMap;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use zeroize::Zeroize;

use crate::{Result, ZeniiError};

use super::CredentialStore;

/// Encrypted file-based credential store.
///
/// Provides persistent credential storage as a fallback when the OS keyring is
/// unavailable. Credentials are stored in a JSON file encrypted with AES-256-GCM,
/// keyed from a machine-derived secret (hostname + username + data_dir + service_id).
///
/// Security model: better than plaintext, comparable to OS keyring for local threats.
/// NOT resistant to root access or memory dumps (same threat model as OS keyring).
pub struct FileCredentialStore {
    path: PathBuf,
    key: Key<Aes256Gcm>,
    // Serializes read-modify-write cycles to prevent concurrent write races.
    // tokio::sync::Mutex is designed to be held across .await points.
    lock: Mutex<()>,
}

impl FileCredentialStore {
    /// Create a new FileCredentialStore at `{data_dir}/credentials.enc`.
    ///
    /// Derives an encryption key from machine characteristics and `service_id`.
    /// Creates the parent directory if needed.
    pub fn new(data_dir: &Path, service_id: &str) -> Result<Self> {
        let path = data_dir.join("credentials.enc");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let key = Self::derive_key(data_dir, service_id);
        Ok(Self {
            path,
            key,
            lock: Mutex::new(()),
        })
    }

    /// Create with a specific file path (for config override or testing).
    pub fn with_path(path: PathBuf, service_id: &str) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data_dir = path.parent().unwrap_or(Path::new("."));
        let key = Self::derive_key(data_dir, service_id);
        Ok(Self {
            path,
            key,
            lock: Mutex::new(()),
        })
    }

    /// Return the path to the encrypted credential file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Derive a 256-bit encryption key from machine characteristics.
    ///
    /// Components: SHA-256(hostname + username + data_dir path + service_id).
    /// Stable across restarts on the same system, but changes if the app moves.
    fn derive_key(data_dir: &Path, service_id: &str) -> Key<Aes256Gcm> {
        let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown-host".into());
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "default-user".into());
        let dir_str = data_dir.to_string_lossy();

        let mut hasher = Sha256::new();
        hasher.update(hostname.as_bytes());
        hasher.update(b":");
        hasher.update(username.as_bytes());
        hasher.update(b":");
        hasher.update(dir_str.as_bytes());
        hasher.update(b":");
        hasher.update(service_id.as_bytes());

        let hash = hasher.finalize();
        let key_bytes: [u8; 32] = hash.into();
        Key::<Aes256Gcm>::from(key_bytes)
    }
}

/// Read and decrypt all credentials from the encrypted file.
/// Returns empty map if file doesn't exist or is too small.
fn read_all_sync(path: &Path, key: &Key<Aes256Gcm>) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let data = std::fs::read(path)
        .map_err(|e| ZeniiError::Credential(format!("failed to read credential file: {e}")))?;

    // File must contain at least 12-byte nonce + some ciphertext
    if data.len() < 13 {
        return Ok(HashMap::new());
    }

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher
        .decrypt(nonce_bytes.into(), ciphertext)
        .map_err(|_| {
            ZeniiError::Credential(
                "failed to decrypt credential file (wrong key or corrupted)".into(),
            )
        })?;

    let mut json_str = String::from_utf8(plaintext)
        .map_err(|_| ZeniiError::Credential("credential file contains invalid UTF-8".into()))?;

    let map: HashMap<String, String> = serde_json::from_str(&json_str)
        .map_err(|e| ZeniiError::Credential(format!("credential file JSON invalid: {e}")))?;

    json_str.zeroize();
    Ok(map)
}

/// Serialize, encrypt, and atomically write all credentials to the file.
/// Uses tmp-file + rename for crash safety. Sets 0o600 permissions on Unix.
fn write_all_sync(path: &Path, key: &Key<Aes256Gcm>, data: &HashMap<String, String>) -> Result<()> {
    let mut json = serde_json::to_string(data)
        .map_err(|e| ZeniiError::Credential(format!("failed to serialize credentials: {e}")))?;

    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, json.as_bytes())
        .map_err(|_| ZeniiError::Credential("failed to encrypt credentials".into()))?;

    json.zeroize();

    // Format: [12-byte nonce][ciphertext]
    let mut output = Vec::with_capacity(12 + ciphertext.len());
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);

    // Atomic write: write to tmp then rename
    let tmp_path = path.with_extension("enc.tmp");
    std::fs::write(&tmp_path, &output)
        .map_err(|e| ZeniiError::Credential(format!("failed to write credential file: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&tmp_path, perms)
            .map_err(|e| ZeniiError::Credential(format!("failed to set file permissions: {e}")))?;
    }

    std::fs::rename(&tmp_path, path)
        .map_err(|e| ZeniiError::Credential(format!("failed to rename credential file: {e}")))?;

    Ok(())
}

#[async_trait]
impl CredentialStore for FileCredentialStore {
    async fn set(&self, key: &str, value: &str) -> Result<()> {
        let _guard = self.lock.lock().await;
        let cred_key = key.to_string();
        let cred_value = value.to_string();
        let path = self.path.clone();
        let enc_key = self.key;

        tokio::task::spawn_blocking(move || {
            let mut map = read_all_sync(&path, &enc_key)?;
            map.insert(cred_key, cred_value);
            write_all_sync(&path, &enc_key, &map)
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }

    async fn get(&self, key: &str) -> Result<Option<String>> {
        let _guard = self.lock.lock().await;
        let cred_key = key.to_string();
        let path = self.path.clone();
        let enc_key = self.key;

        tokio::task::spawn_blocking(move || {
            let map = read_all_sync(&path, &enc_key)?;
            Ok(map.get(&cred_key).cloned())
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let _guard = self.lock.lock().await;
        let cred_key = key.to_string();
        let path = self.path.clone();
        let enc_key = self.key;

        tokio::task::spawn_blocking(move || {
            let mut map = read_all_sync(&path, &enc_key)?;
            let removed = map.remove(&cred_key).is_some();
            if removed {
                write_all_sync(&path, &enc_key, &map)?;
            }
            Ok(removed)
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }

    async fn list(&self) -> Result<Vec<String>> {
        let _guard = self.lock.lock().await;
        let path = self.path.clone();
        let enc_key = self.key;

        tokio::task::spawn_blocking(move || {
            let map = read_all_sync(&path, &enc_key)?;
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();
            Ok(keys)
        })
        .await
        .map_err(|e| ZeniiError::Credential(format!("spawn_blocking error: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store(dir: &Path) -> FileCredentialStore {
        FileCredentialStore::new(dir, "test-service").unwrap()
    }

    #[tokio::test]
    async fn set_and_get() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        store.set("api_key:openai", "sk-test123").await.unwrap();
        assert_eq!(
            store.get("api_key:openai").await.unwrap(),
            Some("sk-test123".to_string())
        );
    }

    #[tokio::test]
    async fn get_missing_key() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        assert_eq!(store.get("nonexistent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_existing() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        store.set("key", "val").await.unwrap();
        assert!(store.delete("key").await.unwrap());
        assert_eq!(store.get("key").await.unwrap(), None);
    }

    #[tokio::test]
    async fn delete_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        assert!(!store.delete("nope").await.unwrap());
    }

    #[tokio::test]
    async fn list_sorted() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        store.set("zebra", "z").await.unwrap();
        store.set("alpha", "a").await.unwrap();
        store.set("middle", "m").await.unwrap();
        let keys = store.list().await.unwrap();
        assert_eq!(keys, vec!["alpha", "middle", "zebra"]);
    }

    #[tokio::test]
    async fn persists_across_instances() {
        let dir = tempfile::TempDir::new().unwrap();

        // First instance: write
        {
            let store = make_store(dir.path());
            store.set("persist_key", "persist_val").await.unwrap();
        }

        // Second instance: read (same path + service_id = same key)
        {
            let store = make_store(dir.path());
            assert_eq!(
                store.get("persist_key").await.unwrap(),
                Some("persist_val".to_string())
            );
        }
    }

    #[tokio::test]
    async fn atomic_write_no_corruption() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        store.set("key", "value").await.unwrap();

        let enc_path = dir.path().join("credentials.enc");
        assert!(enc_path.exists());
        assert!(enc_path.metadata().unwrap().len() > 12);

        // Verify no tmp file lingers
        let tmp_path = dir.path().join("credentials.enc.tmp");
        assert!(!tmp_path.exists());
    }

    #[tokio::test]
    async fn wrong_key_fails() {
        let dir = tempfile::TempDir::new().unwrap();

        // Write with service "alpha"
        let store_a = FileCredentialStore::new(dir.path(), "alpha").unwrap();
        store_a.set("secret", "data").await.unwrap();

        // Read with service "beta" — different key, should fail to decrypt
        let store_b = FileCredentialStore::new(dir.path(), "beta").unwrap();
        let result = store_b.get("secret").await;
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        store.set("key", "val").await.unwrap();

        let enc_path = dir.path().join("credentials.enc");
        let mode = enc_path.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Credential file should be owner-only (0600)");
    }

    #[tokio::test]
    async fn overwrite_value() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = make_store(dir.path());
        store.set("key", "old").await.unwrap();
        store.set("key", "new").await.unwrap();
        assert_eq!(store.get("key").await.unwrap(), Some("new".to_string()));
    }
}
