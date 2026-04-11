//! Zero-JSON settings store for Rustacle.
//!
//! Every setting is persisted in `SQLite`, accessed via typed APIs,
//! and exposed to the UI via IPC. No config files.

pub mod import_export;
pub mod schema;
pub mod secrets;
pub mod store;

pub use schema::SettingKey;
pub use secrets::{KeyringStore, SecretString};
pub use store::SettingsStore;

/// Errors from the settings subsystem.
#[derive(thiserror::Error, Debug)]
pub enum SettingsError {
    #[error("database error: {0}")]
    Database(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("import error: {0}")]
    Import(String),
}
