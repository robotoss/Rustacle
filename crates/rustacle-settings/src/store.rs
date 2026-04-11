//! SQLite-backed typed settings store.
//!
//! Every setting is persisted in `SQLite`, accessed via typed APIs.
//! No config files. Change notifications fire on updates.

use std::path::Path;
use std::sync::Arc;

use rusqlite::{Connection, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;
use tracing::{debug, info};

use crate::SettingsError;
use crate::schema::SettingKey;

/// Schema version for forward-compatible migrations.
const SCHEMA_VERSION: u32 = 1;

/// Change notification emitted when a setting is updated.
#[derive(Debug, Clone)]
pub struct SettingChange {
    pub key: String,
    pub value: serde_json::Value,
}

/// The settings store. Thread-safe via internal locking.
pub struct SettingsStore {
    conn: Arc<std::sync::Mutex<Connection>>,
    change_tx: broadcast::Sender<SettingChange>,
}

impl SettingsStore {
    /// Open or create a settings database at the given path.
    ///
    /// # Errors
    /// Returns `SettingsError` if the database cannot be opened or migrated.
    pub fn open(path: &Path) -> Result<Self, SettingsError> {
        let conn = Connection::open(path).map_err(|e| SettingsError::Database(e.to_string()))?;

        // Run migrations
        Self::migrate(&conn)?;

        let (change_tx, _) = broadcast::channel(64);

        info!(path = %path.display(), "settings store opened");

        Ok(Self {
            conn: Arc::new(std::sync::Mutex::new(conn)),
            change_tx,
        })
    }

    /// Open an in-memory database (for testing).
    ///
    /// # Errors
    /// Returns `SettingsError` if the database cannot be created.
    pub fn open_memory() -> Result<Self, SettingsError> {
        let conn =
            Connection::open_in_memory().map_err(|e| SettingsError::Database(e.to_string()))?;

        Self::migrate(&conn)?;

        let (change_tx, _) = broadcast::channel(64);

        Ok(Self {
            conn: Arc::new(std::sync::Mutex::new(conn)),
            change_tx,
        })
    }

    /// Run schema migrations.
    fn migrate(conn: &Connection) -> Result<(), SettingsError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| SettingsError::Database(format!("migration: {e}")))?;

        // Check/set schema version
        let version: Option<u32> = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .ok();

        if version.is_none() {
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )
            .map_err(|e| SettingsError::Database(format!("set version: {e}")))?;
        }

        debug!(schema_version = SCHEMA_VERSION, "migrations complete");
        Ok(())
    }

    /// Get a typed setting value, returning the default if not set.
    ///
    /// # Errors
    /// Returns `SettingsError` on database or deserialization errors.
    pub fn get<T: DeserializeOwned>(&self, key: SettingKey) -> Result<T, SettingsError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key.as_str()],
                |row| row.get(0),
            )
            .ok();

        let json_value = match result {
            Some(raw) => serde_json::from_str(&raw)
                .map_err(|e| SettingsError::Serialization(e.to_string()))?,
            None => key.default_json(),
        };

        serde_json::from_value(json_value).map_err(|e| SettingsError::Serialization(e.to_string()))
    }

    /// Get a raw JSON value for a setting.
    ///
    /// # Errors
    /// Returns `SettingsError` on database errors.
    pub fn get_json(&self, key: SettingKey) -> Result<serde_json::Value, SettingsError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key.as_str()],
                |row| row.get(0),
            )
            .ok();

        match result {
            Some(raw) => {
                serde_json::from_str(&raw).map_err(|e| SettingsError::Serialization(e.to_string()))
            }
            None => Ok(key.default_json()),
        }
    }

    /// Set a typed setting value, persisting to `SQLite` and emitting a change notification.
    ///
    /// # Errors
    /// Returns `SettingsError` on database or serialization errors.
    pub fn set<T: Serialize>(&self, key: SettingKey, value: &T) -> Result<(), SettingsError> {
        let json =
            serde_json::to_value(value).map_err(|e| SettingsError::Serialization(e.to_string()))?;
        self.set_json(key, json)
    }

    /// Set a raw JSON value.
    ///
    /// # Errors
    /// Returns `SettingsError` on database errors.
    pub fn set_json(&self, key: SettingKey, value: serde_json::Value) -> Result<(), SettingsError> {
        let raw = serde_json::to_string(&value)
            .map_err(|e| SettingsError::Serialization(e.to_string()))?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
            params![key.as_str(), raw],
        )
        .map_err(|e| SettingsError::Database(format!("set {}: {e}", key.as_str())))?;

        debug!(key = key.as_str(), "setting updated");

        // Emit change notification (best-effort)
        let _ = self.change_tx.send(SettingChange {
            key: key.as_str().to_owned(),
            value,
        });

        Ok(())
    }

    /// Batch update: all-or-nothing transaction.
    ///
    /// # Errors
    /// Returns `SettingsError` if any update fails (all rolled back).
    pub fn batch_set(
        &self,
        updates: &[(SettingKey, serde_json::Value)],
    ) -> Result<(), SettingsError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SettingsError::Database(e.to_string()))?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| SettingsError::Database(format!("begin tx: {e}")))?;

        for (key, value) in updates {
            let raw = serde_json::to_string(value)
                .map_err(|e| SettingsError::Serialization(e.to_string()))?;

            tx.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
                params![key.as_str(), raw],
            )
            .map_err(|e| SettingsError::Database(format!("batch set {}: {e}", key.as_str())))?;
        }

        tx.commit()
            .map_err(|e| SettingsError::Database(format!("commit: {e}")))?;

        // Emit change notifications
        for (key, value) in updates {
            let _ = self.change_tx.send(SettingChange {
                key: key.as_str().to_owned(),
                value: value.clone(),
            });
        }

        Ok(())
    }

    /// Subscribe to setting changes.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<SettingChange> {
        self.change_tx.subscribe()
    }

    /// List all settings with their current values (or defaults).
    ///
    /// # Errors
    /// Returns `SettingsError` on database errors.
    pub fn list_all(&self) -> Result<Vec<crate::schema::SettingEntry>, SettingsError> {
        let mut entries = Vec::new();
        for key in SettingKey::ALL {
            let value = self.get_json(*key)?;
            entries.push(crate::schema::SettingEntry {
                key: key.as_str().to_owned(),
                value,
                description: key.description().to_owned(),
            });
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_default_when_unset() {
        let store = SettingsStore::open_memory().unwrap();
        let val: String = store.get(SettingKey::UiTheme).unwrap();
        assert_eq!(val, "dark");
    }

    #[test]
    fn set_and_get_roundtrip() {
        let store = SettingsStore::open_memory().unwrap();
        store.set(SettingKey::UiTheme, &"light").unwrap();
        let val: String = store.get(SettingKey::UiTheme).unwrap();
        assert_eq!(val, "light");
    }

    #[test]
    fn set_overwrites_previous() {
        let store = SettingsStore::open_memory().unwrap();
        store.set(SettingKey::TerminalFontSize, &16).unwrap();
        store.set(SettingKey::TerminalFontSize, &20).unwrap();
        let val: i64 = store.get(SettingKey::TerminalFontSize).unwrap();
        assert_eq!(val, 20);
    }

    #[test]
    fn batch_set_all_or_nothing() {
        let store = SettingsStore::open_memory().unwrap();
        let updates = vec![
            (SettingKey::UiTheme, serde_json::json!("solarized")),
            (SettingKey::TerminalFontSize, serde_json::json!(18)),
        ];
        store.batch_set(&updates).unwrap();

        let theme: String = store.get(SettingKey::UiTheme).unwrap();
        let size: i64 = store.get(SettingKey::TerminalFontSize).unwrap();
        assert_eq!(theme, "solarized");
        assert_eq!(size, 18);
    }

    #[test]
    fn change_notification_fires() {
        let store = SettingsStore::open_memory().unwrap();
        let mut rx = store.subscribe();

        store.set(SettingKey::UiTheme, &"light").unwrap();

        let change = rx.try_recv().unwrap();
        assert_eq!(change.key, "ui.theme");
    }

    #[test]
    fn list_all_returns_all_keys() {
        let store = SettingsStore::open_memory().unwrap();
        let entries = store.list_all().unwrap();
        assert_eq!(entries.len(), SettingKey::ALL.len());
    }

    #[test]
    fn get_json_returns_default() {
        let store = SettingsStore::open_memory().unwrap();
        let val = store.get_json(SettingKey::AgentMaxToolCalls).unwrap();
        assert_eq!(val, serde_json::json!(50));
    }

    #[test]
    fn persistent_store() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("settings.db");

        // Write
        {
            let store = SettingsStore::open(&db_path).unwrap();
            store.set(SettingKey::UiTheme, &"nord").unwrap();
        }

        // Read from a new connection
        {
            let store = SettingsStore::open(&db_path).unwrap();
            let val: String = store.get(SettingKey::UiTheme).unwrap();
            assert_eq!(val, "nord");
        }
    }
}
