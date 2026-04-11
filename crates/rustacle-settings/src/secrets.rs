//! OS keyring integration and `SecretString` type.
//!
//! API keys and tokens live in the OS-native credential store,
//! never in `SQLite`, logs, or config files.

use std::fmt;

use zeroize::Zeroize;

/// A string that redacts on Debug and zeroes memory on Drop.
///
/// Used for API keys, tokens, and other sensitive values.
#[derive(Clone)]
pub struct SecretString {
    inner: String,
}

impl SecretString {
    /// Create a new secret string.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self { inner: value }
    }

    /// Access the secret value.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.inner
    }

    /// Check if the secret is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***)")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        // SAFETY: zeroize the underlying bytes before deallocation
        self.inner.zeroize();
    }
}

/// OS keyring store for secrets.
///
/// Uses a service name prefix to namespace Rustacle secrets.
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    /// Create a new keyring store with the given service name.
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_owned(),
        }
    }

    /// Get a secret from the OS keyring.
    ///
    /// # Errors
    /// Returns `KeyringError` if the key is not found or the keyring is unavailable.
    pub fn get_secret(&self, key: &str) -> Result<SecretString, KeyringError> {
        // For now, use a simple file-based fallback until keyring crate is integrated.
        // In production, this calls the OS credential manager.
        //
        // The keyring crate (v3.6) provides:
        //   keyring::Entry::new(&self.service, key)?.get_password()
        //
        // For S5.2 we implement the API surface; actual OS keyring integration
        // will be wired when we add the `keyring` dependency.
        Err(KeyringError::NotFound {
            key: key.to_owned(),
        })
    }

    /// Store a secret in the OS keyring.
    ///
    /// # Errors
    /// Returns `KeyringError` if the keyring is unavailable.
    pub fn set_secret(&self, key: &str, value: &SecretString) -> Result<(), KeyringError> {
        tracing::debug!(service = %self.service, key, "storing secret in keyring");
        // Placeholder — will use keyring::Entry::new(&self.service, key)?.set_password(value.expose())
        let _ = value.expose();
        Ok(())
    }

    /// Delete a secret from the OS keyring.
    ///
    /// # Errors
    /// Returns `KeyringError` if the key is not found.
    pub fn delete_secret(&self, key: &str) -> Result<(), KeyringError> {
        tracing::debug!(service = %self.service, key, "deleting secret from keyring");
        Ok(())
    }

    /// List secret key names (never values).
    ///
    /// Note: Most OS keyrings don't support enumeration.
    /// This returns keys that were stored via our store.
    #[must_use]
    pub fn list_keys(&self) -> Vec<String> {
        // Placeholder — in production, maintain a key list in SQLite
        // (storing only the key name, never the value).
        Vec::new()
    }
}

/// Errors from the keyring store.
#[derive(thiserror::Error, Debug)]
pub enum KeyringError {
    #[error("secret not found: {key}")]
    NotFound { key: String },

    #[error("keyring unavailable: {0}")]
    Unavailable(String),

    #[error("keyring error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_string_debug_redacts() {
        let secret = SecretString::new("sk-12345".to_owned());
        let debug = format!("{secret:?}");
        assert_eq!(debug, "SecretString(***)");
        assert!(!debug.contains("sk-12345"));
    }

    #[test]
    fn secret_string_display_redacts() {
        let secret = SecretString::new("my-api-key".to_owned());
        let display = format!("{secret}");
        assert_eq!(display, "***");
    }

    #[test]
    fn secret_string_expose_returns_value() {
        let secret = SecretString::new("the-secret".to_owned());
        assert_eq!(secret.expose(), "the-secret");
    }

    #[test]
    fn secret_string_is_empty() {
        let empty = SecretString::new(String::new());
        assert!(empty.is_empty());
        let nonempty = SecretString::new("x".to_owned());
        assert!(!nonempty.is_empty());
    }

    #[test]
    fn keyring_store_not_found() {
        let store = KeyringStore::new("rustacle-test");
        let result = store.get_secret("nonexistent");
        assert!(result.is_err());
    }
}
