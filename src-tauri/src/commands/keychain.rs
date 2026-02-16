use keyring::Entry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KeychainError {
    message: String,
}

impl From<keyring::Error> for KeychainError {
    fn from(err: keyring::Error) -> Self {
        KeychainError {
            message: err.to_string(),
        }
    }
}

/// Set a secret in the OS keychain
#[tauri::command]
pub fn keychain_set(service: String, key: String, value: String) -> Result<(), KeychainError> {
    let entry = Entry::new(&service, &key)?;
    entry.set_password(&value)?;
    Ok(())
}

/// Get a secret from the OS keychain
#[tauri::command]
pub fn keychain_get(service: String, key: String) -> Result<String, KeychainError> {
    let entry = Entry::new(&service, &key)?;
    let password = entry.get_password()?;
    Ok(password)
}

/// Delete a secret from the OS keychain
#[tauri::command]
pub fn keychain_delete(service: String, key: String) -> Result<(), KeychainError> {
    let entry = Entry::new(&service, &key)?;
    entry.delete_password()?;
    Ok(())
}

/// Check if a secret exists in the OS keychain
#[tauri::command]
pub fn keychain_exists(service: String, key: String) -> Result<bool, KeychainError> {
    let entry = Entry::new(&service, &key)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(e.into()),
    }
}
