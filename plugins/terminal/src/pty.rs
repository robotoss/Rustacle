use std::io::{Read, Write};
use std::sync::Mutex;

use portable_pty::{Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use rustacle_plugin_api::ModuleError;

/// A single PTY session (one tab = one session).
///
/// Uses `Mutex` wrappers because `MasterPty` and `Read` are not `Sync`,
/// but `RustacleModule` requires `Send + Sync`.
pub struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
    reader: Mutex<Box<dyn Read + Send>>,
}

impl PtySession {
    /// Spawn a new PTY session with the user's default shell.
    ///
    /// # Errors
    /// Returns `ModuleError::Internal` if shell detection or PTY creation fails.
    pub fn spawn(cwd: Option<&str>, cols: u16, rows: u16) -> Result<Self, ModuleError> {
        let pty_system = NativePtySystem::default();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ModuleError::Internal(format!("failed to open PTY: {e}")))?;

        let shell = detect_shell();
        let mut cmd = CommandBuilder::new(&shell);

        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| ModuleError::Internal(format!("failed to spawn shell '{shell}': {e}")))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| ModuleError::Internal(format!("failed to clone PTY reader: {e}")))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| ModuleError::Internal(format!("failed to take PTY writer: {e}")))?;

        tracing::info!(shell = %shell, cols, rows, "PTY session spawned");

        Ok(Self {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            child: Mutex::new(child),
            reader: Mutex::new(reader),
        })
    }

    /// Write input bytes to the PTY.
    ///
    /// # Errors
    /// Returns `ModuleError::Internal` on write failure.
    ///
    /// # Panics
    /// Panics if the PTY writer mutex is poisoned.
    pub fn write(&self, data: &[u8]) -> Result<(), ModuleError> {
        // INVARIANT: lock never poisoned in normal operation
        let mut writer = self.writer.lock().expect("PTY writer lock poisoned");
        writer
            .write_all(data)
            .map_err(|e| ModuleError::Internal(format!("PTY write failed: {e}")))
    }

    /// Read available output from the PTY.
    ///
    /// # Errors
    /// Returns `ModuleError::Internal` on read failure.
    ///
    /// # Panics
    /// Panics if the PTY reader mutex is poisoned.
    pub fn read(&self) -> Result<Vec<u8>, ModuleError> {
        let mut reader = self.reader.lock().expect("PTY reader lock poisoned");
        let mut buf = vec![0u8; 4096];
        match reader.read(&mut buf) {
            Ok(0) => Ok(vec![]),
            Ok(n) => {
                buf.truncate(n);
                Ok(buf)
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(vec![]),
            Err(e) => Err(ModuleError::Internal(format!("PTY read failed: {e}"))),
        }
    }

    /// Resize the PTY.
    ///
    /// # Errors
    /// Returns `ModuleError::Internal` on resize failure.
    ///
    /// # Panics
    /// Panics if the PTY master mutex is poisoned.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), ModuleError> {
        let master = self.master.lock().expect("PTY master lock poisoned");
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ModuleError::Internal(format!("PTY resize failed: {e}")))
    }

    /// Check if the child process is still running.
    ///
    /// # Panics
    /// Panics if the PTY child mutex is poisoned.
    pub fn is_alive(&self) -> bool {
        let mut child = self.child.lock().expect("PTY child lock poisoned");
        child.try_wait().ok().flatten().is_none()
    }

    /// Kill the child process.
    pub fn kill(&self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Detect the user's default shell.
fn detect_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }

    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_returns_something() {
        let shell = detect_shell();
        assert!(!shell.is_empty(), "shell should not be empty");
    }

    #[test]
    fn pty_spawn_and_alive() {
        let session = PtySession::spawn(None, 80, 24).expect("PTY should spawn");
        assert!(session.is_alive(), "child should be alive after spawn");
    }
}
