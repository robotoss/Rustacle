use serde::{Deserialize, Serialize};

/// State migration policy declared in a plugin's manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum StateMigrationPolicy {
    /// State is discarded on hot-swap (default for leaf plugins like `fs`).
    #[default]
    Transient,

    /// Plugin implements `export_state`/`import_state`; host enforces max size.
    Serialized { max_bytes: usize },

    /// State lives outside the plugin (in `SQLite`); swap is trivial.
    ExternalStore,
}
