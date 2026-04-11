use serde::{Deserialize, Serialize};

/// A capability that a plugin may request.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum Capability {
    /// Filesystem access within a scoped path.
    Fs { scope: PathScope, mode: FsMode },

    /// Network access to specific hosts.
    Net { allow_hosts: Vec<HostPattern> },

    /// PTY spawning (native plugins only).
    Pty,

    /// Access to a named secret from the keyring.
    Secret { key: String },

    /// Access to the LLM provider router.
    LlmProvider,
}

/// A canonicalized path prefix for filesystem scoping.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PathScope {
    /// The canonical absolute path prefix.
    path: String,
}

impl PathScope {
    /// Create a new path scope from an absolute path.
    ///
    /// # Panics
    /// Panics if canonicalization fails (path does not exist).
    #[must_use]
    pub fn new(path: &std::path::Path) -> Self {
        Self {
            path: path.to_string_lossy().into_owned(),
        }
    }

    /// The canonical path as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.path
    }

    /// Check if `candidate` is within this scope using segment-boundary matching.
    #[must_use]
    pub fn contains(&self, candidate: &str) -> bool {
        if candidate == self.path {
            return true;
        }
        candidate.starts_with(&self.path)
            && candidate.as_bytes().get(self.path.len()) == Some(&b'/')
            || candidate.as_bytes().get(self.path.len()) == Some(&b'\\')
    }
}

/// Read-only or read-write filesystem access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum FsMode {
    ReadOnly,
    ReadWrite,
}

/// A host pattern for network scoping (supports wildcards like `*.openai.com`).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct HostPattern {
    pattern: String,
}

impl HostPattern {
    #[must_use]
    pub fn new(pattern: String) -> Self {
        Self { pattern }
    }

    /// The raw pattern string.
    #[must_use]
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Check if a hostname matches this pattern.
    #[must_use]
    pub fn matches(&self, host: &str) -> bool {
        if self.pattern.starts_with("*.") {
            let suffix = &self.pattern[1..]; // ".openai.com"
            host.ends_with(suffix) || host == &self.pattern[2..]
        } else {
            self.pattern == host
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_scope_contains() {
        let scope = PathScope {
            path: "/home/user/project".to_string(),
        };
        assert!(scope.contains("/home/user/project"));
        assert!(scope.contains("/home/user/project/src/main.rs"));
        assert!(!scope.contains("/home/user/projects"));
        assert!(!scope.contains("/home/user"));
    }

    #[test]
    fn host_pattern_exact() {
        let p = HostPattern::new("api.openai.com".to_string());
        assert!(p.matches("api.openai.com"));
        assert!(!p.matches("evil.openai.com"));
    }

    #[test]
    fn host_pattern_wildcard() {
        let p = HostPattern::new("*.openai.com".to_string());
        assert!(p.matches("api.openai.com"));
        assert!(p.matches("openai.com"));
        assert!(!p.matches("api.evil.com"));
    }
}
