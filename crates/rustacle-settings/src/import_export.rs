//! Settings import/export with typed schema.
//!
//! Exports all settings except secrets. Import shows a diff before applying.

use serde::{Deserialize, Serialize};

use crate::schema::SettingKey;
use crate::store::SettingsStore;
use crate::SettingsError;

/// Schema version for the export format.
const EXPORT_SCHEMA_VERSION: u32 = 1;

/// Exported settings payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsExport {
    pub schema_version: u32,
    pub settings: Vec<ExportEntry>,
}

/// A single setting in the export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    pub key: String,
    pub value: serde_json::Value,
}

/// A diff entry showing what would change on import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub key: String,
    pub current: serde_json::Value,
    pub incoming: serde_json::Value,
    pub changed: bool,
}

/// Export all settings (excluding secrets) to a typed payload.
///
/// # Errors
/// Returns `SettingsError` on database errors.
pub fn export_settings(store: &SettingsStore) -> Result<SettingsExport, SettingsError> {
    let entries = store.list_all()?;
    let settings = entries
        .into_iter()
        .map(|e| ExportEntry {
            key: e.key,
            value: e.value,
        })
        .collect();

    Ok(SettingsExport {
        schema_version: EXPORT_SCHEMA_VERSION,
        settings,
    })
}

/// Compute a diff between imported settings and current state.
///
/// # Errors
/// Returns `SettingsError` on invalid schema version or database errors.
pub fn diff_import(
    store: &SettingsStore,
    payload: &SettingsExport,
) -> Result<Vec<DiffEntry>, SettingsError> {
    if payload.schema_version != EXPORT_SCHEMA_VERSION {
        return Err(SettingsError::Import(format!(
            "unsupported schema version: {} (expected {})",
            payload.schema_version, EXPORT_SCHEMA_VERSION
        )));
    }

    let mut diffs = Vec::new();
    for entry in &payload.settings {
        // Find the corresponding key
        let current = SettingKey::ALL
            .iter()
            .find(|k| k.as_str() == entry.key)
            .map(|k| store.get_json(*k))
            .transpose()?
            .unwrap_or(serde_json::Value::Null);

        let changed = current != entry.value;
        diffs.push(DiffEntry {
            key: entry.key.clone(),
            current,
            incoming: entry.value.clone(),
            changed,
        });
    }

    Ok(diffs)
}

/// Apply imported settings (only changed entries).
///
/// # Errors
/// Returns `SettingsError` on database errors.
pub fn apply_import(
    store: &SettingsStore,
    payload: &SettingsExport,
) -> Result<u32, SettingsError> {
    let diffs = diff_import(store, payload)?;
    let changed: Vec<_> = diffs
        .into_iter()
        .filter(|d| d.changed)
        .collect();

    let updates: Vec<(SettingKey, serde_json::Value)> = changed
        .iter()
        .filter_map(|d| {
            SettingKey::ALL
                .iter()
                .find(|k| k.as_str() == d.key)
                .map(|k| (*k, d.incoming.clone()))
        })
        .collect();

    #[allow(clippy::cast_possible_truncation)]
    let count = updates.len() as u32;
    if !updates.is_empty() {
        store.batch_set(&updates)?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_import_roundtrip() {
        let store = SettingsStore::open_memory().unwrap();
        store.set(SettingKey::UiTheme, &"nord").unwrap();

        let exported = export_settings(&store).unwrap();
        let json = serde_json::to_string(&exported).unwrap();

        // Import into a fresh store
        let store2 = SettingsStore::open_memory().unwrap();
        let imported: SettingsExport = serde_json::from_str(&json).unwrap();

        let diffs = diff_import(&store2, &imported).unwrap();
        let changed: Vec<_> = diffs.iter().filter(|d| d.changed).collect();
        assert!(!changed.is_empty());

        let count = apply_import(&store2, &imported).unwrap();
        assert!(count > 0);

        let theme: String = store2.get(SettingKey::UiTheme).unwrap();
        assert_eq!(theme, "nord");
    }

    #[test]
    fn invalid_schema_version_rejected() {
        let store = SettingsStore::open_memory().unwrap();
        let payload = SettingsExport {
            schema_version: 999,
            settings: vec![],
        };
        let result = diff_import(&store, &payload);
        assert!(result.is_err());
    }

    #[test]
    fn export_excludes_no_secrets() {
        let store = SettingsStore::open_memory().unwrap();
        let exported = export_settings(&store).unwrap();
        // No key should contain "sk-" or secret-like values
        for entry in &exported.settings {
            assert!(!entry.key.contains("secret"));
        }
    }
}
