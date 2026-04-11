use rustacle_plugin_api::Capability;

/// A normalized cache key for capability lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CapabilityKey {
    Fs { path: String, read_write: bool },
    Net { host: String },
    Pty,
    Secret { key: String },
    LlmProvider,
}

impl From<&Capability> for CapabilityKey {
    fn from(cap: &Capability) -> Self {
        match cap {
            Capability::Fs { scope, mode } => Self::Fs {
                path: scope.as_str().to_string(),
                read_write: *mode == rustacle_plugin_api::FsMode::ReadWrite,
            },
            Capability::Net { allow_hosts } => Self::Net {
                host: allow_hosts
                    .first()
                    .map_or_else(String::new, |h| h.pattern().to_string()),
            },
            Capability::Pty => Self::Pty,
            Capability::Secret { key } => Self::Secret { key: key.clone() },
            Capability::LlmProvider => Self::LlmProvider,
        }
    }
}
