use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

/// Service name for keychain entries
const SERVICE_NAME: &str = "com.aiboilerplate.credentials";

/// Credential types supported by the store
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CredentialType {
    /// Vercel AI Gateway API key
    VercelAIGateway,
    /// Database connection password
    DatabasePassword,
    /// Generic API key
    ApiKey,
    /// SSH password for tunnel authentication
    SshPassword,
    /// SSH private key passphrase
    SshPassphrase,
}

impl CredentialType {
    /// Convert credential type to service identifier
    fn to_service_id(&self) -> String {
        match self {
            CredentialType::VercelAIGateway => format!("{}.vercel-ai-gateway", SERVICE_NAME),
            CredentialType::DatabasePassword => format!("{}.database-password", SERVICE_NAME),
            CredentialType::ApiKey => format!("{}.api-key", SERVICE_NAME),
            CredentialType::SshPassword => format!("{}.ssh-password", SERVICE_NAME),
            CredentialType::SshPassphrase => format!("{}.ssh-passphrase", SERVICE_NAME),
        }
    }
}

/// Errors that can occur during credential operations
#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Keyring error: {0}")]
    KeyringError(#[from] KeyringError),

    #[error("Credential not found for type: {0:?}, username: {1}")]
    NotFound(CredentialType, String),

    #[error("Invalid credential type")]
    InvalidCredentialType,
}

/// Credential store service for secure credential management
pub struct CredentialStore;

impl CredentialStore {
    /// Create a new credential store instance
    pub fn new() -> Self {
        Self
    }

    /// Save credentials to OS keychain
    ///
    /// # Arguments
    /// * `credential_type` - Type of credential to store
    /// * `username` - Username or identifier for the credential
    /// * `password` - Password or secret to store
    ///
    /// # Example
    /// ```no_run
    /// use local_ts_lib::services::credential_store::{CredentialStore, CredentialType};
    ///
    /// let store = CredentialStore::new();
    /// store.save_credentials(
    ///     CredentialType::VercelAIGateway,
    ///     "default",
    ///     "sk-1234567890abcdef"
    /// ).expect("Failed to save credentials");
    /// ```
    pub fn save_credentials(
        &self,
        credential_type: CredentialType,
        username: &str,
        password: &str,
    ) -> Result<(), CredentialError> {
        let service = credential_type.to_service_id();
        let entry = Entry::new(&service, username)?;
        entry.set_password(password)?;
        Ok(())
    }

    /// Get credentials from OS keychain
    ///
    /// # Arguments
    /// * `credential_type` - Type of credential to retrieve
    /// * `username` - Username or identifier for the credential
    ///
    /// # Returns
    /// The password/secret as a String. The caller is responsible for
    /// securely clearing the string when done.
    ///
    /// # Example
    /// ```no_run
    /// use local_ts_lib::services::credential_store::{CredentialStore, CredentialType};
    ///
    /// let store = CredentialStore::new();
    /// let api_key = store.get_credentials(
    ///     CredentialType::VercelAIGateway,
    ///     "default"
    /// ).expect("Failed to get credentials");
    /// ```
    pub fn get_credentials(
        &self,
        credential_type: CredentialType,
        username: &str,
    ) -> Result<String, CredentialError> {
        let service = credential_type.to_service_id();
        let entry = Entry::new(&service, username)?;

        match entry.get_password() {
            Ok(password) => Ok(password),
            Err(KeyringError::NoEntry) => Err(CredentialError::NotFound(
                credential_type,
                username.to_string(),
            )),
            Err(e) => Err(CredentialError::KeyringError(e)),
        }
    }

    /// Delete credentials from OS keychain
    ///
    /// # Arguments
    /// * `credential_type` - Type of credential to delete
    /// * `username` - Username or identifier for the credential
    ///
    /// # Example
    /// ```no_run
    /// use local_ts_lib::services::credential_store::{CredentialStore, CredentialType};
    ///
    /// let store = CredentialStore::new();
    /// store.delete_credentials(
    ///     CredentialType::VercelAIGateway,
    ///     "default"
    /// ).expect("Failed to delete credentials");
    /// ```
    pub fn delete_credentials(
        &self,
        credential_type: CredentialType,
        username: &str,
    ) -> Result<(), CredentialError> {
        let service = credential_type.to_service_id();
        let entry = Entry::new(&service, username)?;

        match entry.delete_password() {
            Ok(_) => Ok(()),
            Err(KeyringError::NoEntry) => Err(CredentialError::NotFound(
                credential_type,
                username.to_string(),
            )),
            Err(e) => Err(CredentialError::KeyringError(e)),
        }
    }

    /// Check if credentials exist in OS keychain
    ///
    /// # Arguments
    /// * `credential_type` - Type of credential to check
    /// * `username` - Username or identifier for the credential
    ///
    /// # Returns
    /// `true` if credentials exist, `false` otherwise
    ///
    /// # Example
    /// ```no_run
    /// use local_ts_lib::services::credential_store::{CredentialStore, CredentialType};
    ///
    /// let store = CredentialStore::new();
    /// if store.credential_exists(CredentialType::VercelAIGateway, "default") {
    ///     println!("API key is configured");
    /// }
    /// ```
    pub fn credential_exists(&self, credential_type: CredentialType, username: &str) -> bool {
        let service = credential_type.to_service_id();
        if let Ok(entry) = Entry::new(&service, username) {
            entry.get_password().is_ok()
        } else {
            false
        }
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Securely clear a string containing sensitive data
///
/// # Example
/// ```
/// use local_ts_lib::services::credential_store::secure_clear;
///
/// let mut api_key = String::from("sk-1234567890abcdef");
/// secure_clear(&mut api_key);
/// assert_eq!(api_key, "");
/// ```
pub fn secure_clear(s: &mut String) {
    s.zeroize();
}

/// Helper functions for SSH credential management
impl CredentialStore {
    /// Save SSH password to keyring
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace identifier for the credential
    /// * `password` - SSH password to store
    ///
    /// # Example
    /// ```no_run
    /// use local_ts_lib::services::credential_store::CredentialStore;
    ///
    /// let store = CredentialStore::new();
    /// store.save_ssh_password("workspace-123", "my-ssh-password")
    ///     .expect("Failed to save SSH password");
    /// ```
    pub fn save_ssh_password(&self, workspace_id: &str, password: &str) -> Result<(), CredentialError> {
        self.save_credentials(CredentialType::SshPassword, workspace_id, password)
    }

    /// Get SSH password from keyring
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace identifier for the credential
    ///
    /// # Returns
    /// The SSH password as a String, or an error if not found
    pub fn get_ssh_password(&self, workspace_id: &str) -> Result<String, CredentialError> {
        self.get_credentials(CredentialType::SshPassword, workspace_id)
    }

    /// Delete SSH password from keyring
    pub fn delete_ssh_password(&self, workspace_id: &str) -> Result<(), CredentialError> {
        self.delete_credentials(CredentialType::SshPassword, workspace_id)
    }

    /// Save SSH passphrase to keyring
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace identifier for the credential
    /// * `passphrase` - SSH key passphrase to store
    pub fn save_ssh_passphrase(&self, workspace_id: &str, passphrase: &str) -> Result<(), CredentialError> {
        self.save_credentials(CredentialType::SshPassphrase, workspace_id, passphrase)
    }

    /// Get SSH passphrase from keyring
    pub fn get_ssh_passphrase(&self, workspace_id: &str) -> Result<String, CredentialError> {
        self.get_credentials(CredentialType::SshPassphrase, workspace_id)
    }

    /// Delete SSH passphrase from keyring
    pub fn delete_ssh_passphrase(&self, workspace_id: &str) -> Result<(), CredentialError> {
        self.delete_credentials(CredentialType::SshPassphrase, workspace_id)
    }

    /// Check if SSH credentials exist for a workspace
    pub fn ssh_credentials_exist(&self, workspace_id: &str) -> (bool, bool) {
        (
            self.credential_exists(CredentialType::SshPassword, workspace_id),
            self.credential_exists(CredentialType::SshPassphrase, workspace_id),
        )
    }

    /// Save database password to keyring
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace identifier for the credential
    /// * `password` - Database password to store
    ///
    /// # Example
    /// ```no_run
    /// use local_ts_lib::services::credential_store::CredentialStore;
    ///
    /// let store = CredentialStore::new();
    /// store.save_database_password("workspace-123", "my-db-password")
    ///     .expect("Failed to save database password");
    /// ```
    pub fn save_database_password(&self, workspace_id: &str, password: &str) -> Result<(), CredentialError> {
        self.save_credentials(CredentialType::DatabasePassword, workspace_id, password)
    }

    /// Get database password from keyring
    ///
    /// # Arguments
    /// * `workspace_id` - Workspace identifier for the credential
    ///
    /// # Returns
    /// The database password as a String, or an error if not found
    pub fn get_database_password(&self, workspace_id: &str) -> Result<String, CredentialError> {
        self.get_credentials(CredentialType::DatabasePassword, workspace_id)
    }

    /// Delete database password from keyring
    pub fn delete_database_password(&self, workspace_id: &str) -> Result<(), CredentialError> {
        self.delete_credentials(CredentialType::DatabasePassword, workspace_id)
    }

    /// Check if database password exists for a workspace
    pub fn database_password_exists(&self, workspace_id: &str) -> bool {
        self.credential_exists(CredentialType::DatabasePassword, workspace_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to generate unique test usernames
    fn test_username(suffix: &str) -> String {
        format!("test_user_{}", suffix)
    }

    #[test]
    fn test_save_and_get_credentials() {
        let store = CredentialStore::new();
        let username = test_username("save_get");
        let password = "test_password_123";

        // Save credentials
        store
            .save_credentials(CredentialType::VercelAIGateway, &username, password)
            .expect("Failed to save credentials");

        // Retrieve credentials
        let retrieved = store
            .get_credentials(CredentialType::VercelAIGateway, &username)
            .expect("Failed to get credentials");

        assert_eq!(retrieved, password);

        // Cleanup
        let _ = store.delete_credentials(CredentialType::VercelAIGateway, &username);
    }

    #[test]
    fn test_delete_credentials() {
        let store = CredentialStore::new();
        let username = test_username("delete");
        let password = "test_password_456";

        // Save credentials
        store
            .save_credentials(CredentialType::ApiKey, &username, password)
            .expect("Failed to save credentials");

        // Verify they exist
        assert!(store.credential_exists(CredentialType::ApiKey, &username));

        // Delete credentials
        store
            .delete_credentials(CredentialType::ApiKey, &username)
            .expect("Failed to delete credentials");

        // Verify they're gone
        assert!(!store.credential_exists(CredentialType::ApiKey, &username));
    }

    #[test]
    fn test_get_nonexistent_credentials() {
        let store = CredentialStore::new();
        let username = test_username("nonexistent");

        // Try to get credentials that don't exist
        let result = store.get_credentials(CredentialType::DatabasePassword, &username);

        assert!(result.is_err());
        match result {
            Err(CredentialError::NotFound(cred_type, user)) => {
                assert_eq!(cred_type, CredentialType::DatabasePassword);
                assert_eq!(user, username);
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_credential_exists() {
        let store = CredentialStore::new();
        let username = test_username("exists");
        let password = "test_password_789";

        // Should not exist initially
        assert!(!store.credential_exists(CredentialType::VercelAIGateway, &username));

        // Save credentials
        store
            .save_credentials(CredentialType::VercelAIGateway, &username, password)
            .expect("Failed to save credentials");

        // Should exist now
        assert!(store.credential_exists(CredentialType::VercelAIGateway, &username));

        // Cleanup
        let _ = store.delete_credentials(CredentialType::VercelAIGateway, &username);
    }

    #[test]
    fn test_overwrite_credentials() {
        let store = CredentialStore::new();
        let username = test_username("overwrite");
        let password1 = "old_password";
        let password2 = "new_password";

        // Save initial credentials
        store
            .save_credentials(CredentialType::ApiKey, &username, password1)
            .expect("Failed to save initial credentials");

        // Overwrite with new credentials
        store
            .save_credentials(CredentialType::ApiKey, &username, password2)
            .expect("Failed to overwrite credentials");

        // Verify new credentials
        let retrieved = store
            .get_credentials(CredentialType::ApiKey, &username)
            .expect("Failed to get credentials");

        assert_eq!(retrieved, password2);

        // Cleanup
        let _ = store.delete_credentials(CredentialType::ApiKey, &username);
    }

    #[test]
    fn test_secure_clear() {
        let mut sensitive_data = String::from("sk-1234567890abcdef");
        secure_clear(&mut sensitive_data);
        assert_eq!(sensitive_data, "");
    }

    #[test]
    fn test_save_and_get_ssh_password() {
        let store = CredentialStore::new();
        let workspace_id = test_username("ssh_password");
        let password = "test_ssh_password";

        // Save SSH password
        store
            .save_ssh_password(&workspace_id, password)
            .expect("Failed to save SSH password");

        // Retrieve SSH password
        let retrieved = store
            .get_ssh_password(&workspace_id)
            .expect("Failed to get SSH password");

        assert_eq!(retrieved, password);

        // Cleanup
        let _ = store.delete_ssh_password(&workspace_id);
    }

    #[test]
    fn test_save_and_get_ssh_passphrase() {
        let store = CredentialStore::new();
        let workspace_id = test_username("ssh_passphrase");
        let passphrase = "test_ssh_passphrase";

        // Save SSH passphrase
        store
            .save_ssh_passphrase(&workspace_id, passphrase)
            .expect("Failed to save SSH passphrase");

        // Retrieve SSH passphrase
        let retrieved = store
            .get_ssh_passphrase(&workspace_id)
            .expect("Failed to get SSH passphrase");

        assert_eq!(retrieved, passphrase);

        // Cleanup
        let _ = store.delete_ssh_passphrase(&workspace_id);
    }

    #[test]
    fn test_delete_ssh_credentials() {
        let store = CredentialStore::new();
        let workspace_id = test_username("delete_ssh");

        // Save SSH password and passphrase
        store
            .save_ssh_password(&workspace_id, "password")
            .expect("Failed to save SSH password");
        store
            .save_ssh_passphrase(&workspace_id, "passphrase")
            .expect("Failed to save SSH passphrase");

        // Verify they exist
        let (has_password, has_passphrase) = store.ssh_credentials_exist(&workspace_id);
        assert!(has_password);
        assert!(has_passphrase);

        // Delete SSH password
        store
            .delete_ssh_password(&workspace_id)
            .expect("Failed to delete SSH password");

        // Verify only password is deleted
        let (has_password_after, has_passphrase_after) = store.ssh_credentials_exist(&workspace_id);
        assert!(!has_password_after);
        assert!(has_passphrase_after);

        // Delete SSH passphrase
        store
            .delete_ssh_passphrase(&workspace_id)
            .expect("Failed to delete SSH passphrase");

        // Verify both are deleted
        let (has_password_final, has_passphrase_final) = store.ssh_credentials_exist(&workspace_id);
        assert!(!has_password_final);
        assert!(!has_passphrase_final);
    }

    #[test]
    fn test_get_nonexistent_ssh_credentials() {
        let store = CredentialStore::new();
        let workspace_id = test_username("nonexistent_ssh");

        // Try to get SSH password that doesn't exist
        let result = store.get_ssh_password(&workspace_id);
        assert!(result.is_err());

        // Try to get SSH passphrase that doesn't exist
        let result = store.get_ssh_passphrase(&workspace_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_get_database_password() {
        let store = CredentialStore::new();
        let workspace_id = test_username("db_password");
        let password = "test_db_password";

        // Save database password
        store
            .save_database_password(&workspace_id, password)
            .expect("Failed to save database password");

        // Retrieve database password
        let retrieved = store
            .get_database_password(&workspace_id)
            .expect("Failed to get database password");

        assert_eq!(retrieved, password);

        // Cleanup
        let _ = store.delete_database_password(&workspace_id);
    }

    #[test]
    fn test_delete_database_password() {
        let store = CredentialStore::new();
        let workspace_id = test_username("delete_db");

        // Save database password
        store
            .save_database_password(&workspace_id, "password")
            .expect("Failed to save database password");

        // Verify it exists
        assert!(store.database_password_exists(&workspace_id));

        // Delete database password
        store
            .delete_database_password(&workspace_id)
            .expect("Failed to delete database password");

        // Verify it's deleted
        assert!(!store.database_password_exists(&workspace_id));
    }

    #[test]
    fn test_get_nonexistent_database_password() {
        let store = CredentialStore::new();
        let workspace_id = test_username("nonexistent_db");

        // Try to get database password that doesn't exist
        let result = store.get_database_password(&workspace_id);
        assert!(result.is_err());
    }
}
