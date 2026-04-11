use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::errors::KernelError;

/// The micro-kernel core. Owns lifecycle, task set, and shutdown signal.
pub struct Kernel {
    pub shutdown: CancellationToken,
    tasks: JoinSet<()>,
}

impl Kernel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shutdown: CancellationToken::new(),
            tasks: JoinSet::new(),
        }
    }

    /// Start the kernel. Future sprints will discover and load plugins here.
    ///
    /// # Errors
    /// Returns `KernelError::Lifecycle` if startup fails.
    #[allow(clippy::unused_async)] // Will await plugin discovery in Sprint 2
    pub async fn start(&mut self) -> Result<(), KernelError> {
        tracing::info!("kernel starting");
        // Future: discover + load plugins
        tracing::info!("kernel started");
        Ok(())
    }

    /// Gracefully stop the kernel. Cancels all tasks and awaits their completion.
    ///
    /// # Errors
    /// Returns `KernelError::Lifecycle` if shutdown fails.
    pub async fn stop(&mut self) -> Result<(), KernelError> {
        tracing::info!("kernel stopping");
        self.shutdown.cancel();
        while self.tasks.join_next().await.is_some() {}
        tracing::info!("kernel stopped");
        Ok(())
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kernel_start_stop() {
        let mut kernel = Kernel::new();
        kernel.start().await.expect("start should succeed");
        kernel.stop().await.expect("stop should succeed");
        assert!(kernel.shutdown.is_cancelled());
    }
}
