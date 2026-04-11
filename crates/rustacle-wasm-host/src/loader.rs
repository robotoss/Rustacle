use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};

/// Loads and verifies WASM plugin components.
pub struct PluginLoader {
    trusted_keys: Vec<VerifyingKey>,
}

impl PluginLoader {
    /// Create a loader with a set of trusted Ed25519 public keys.
    #[must_use]
    pub fn new(trusted_keys: Vec<VerifyingKey>) -> Self {
        Self { trusted_keys }
    }

    /// Load and verify a `.wasm` file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read, the signature is missing or invalid,
    /// or the WASM binary is malformed.
    pub fn load(&self, wasm_path: &Path, sig_path: &Path) -> Result<Vec<u8>, LoadError> {
        let wasm_bytes = std::fs::read(wasm_path)
            .map_err(|e| LoadError::Io(format!("{}: {e}", wasm_path.display())))?;

        let sig_bytes = std::fs::read(sig_path)
            .map_err(|_| LoadError::UnsignedPlugin(wasm_path.display().to_string()))?;

        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|e| LoadError::InvalidSignature(format!("bad signature format: {e}")))?;

        let verified = self.trusted_keys.iter().any(|key| {
            use ed25519_dalek::Verifier;
            key.verify(&wasm_bytes, &signature).is_ok()
        });

        if !verified {
            return Err(LoadError::InvalidSignature(
                "no trusted key matched".to_string(),
            ));
        }

        tracing::info!(path = %wasm_path.display(), "plugin signature verified");
        Ok(wasm_bytes)
    }
}

/// Errors during plugin loading.
#[derive(thiserror::Error, Debug)]
pub enum LoadError {
    #[error("io error: {0}")]
    Io(String),

    #[error("unsigned plugin: {0}")]
    UnsignedPlugin(String),

    #[error("invalid signature: {0}")]
    InvalidSignature(String),

    #[error("wasm error: {0}")]
    Wasm(String),
}
