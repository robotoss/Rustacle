//! Cancellation token wiring for turns and tool calls.

use tokio_util::sync::CancellationToken;

/// Handle for cancelling a running turn.
///
/// The UI Stop button calls [`CancelHandle::cancel`]; every `.await` in the
/// harness is `tokio::select!`'d against the token.
#[derive(Clone)]
pub struct CancelHandle {
    token: CancellationToken,
}

impl CancelHandle {
    /// Create a new cancel handle.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Cancel the turn. All child tokens are also cancelled.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Check if cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Get the underlying token (for `tokio::select!`).
    #[must_use]
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    /// Create a child token for a tool call. Cancelling the parent
    /// cancels the child, but not vice versa.
    #[must_use]
    pub fn child(&self) -> CancellationToken {
        self.token.child_token()
    }
}

impl Default for CancelHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_propagates_to_child() {
        let handle = CancelHandle::new();
        let child = handle.child();

        assert!(!handle.is_cancelled());
        assert!(!child.is_cancelled());

        handle.cancel();

        assert!(handle.is_cancelled());
        assert!(child.is_cancelled());
    }

    #[test]
    fn child_cancel_does_not_propagate_up() {
        let handle = CancelHandle::new();
        let child = handle.child();

        child.cancel();

        assert!(!handle.is_cancelled());
        assert!(child.is_cancelled());
    }
}
