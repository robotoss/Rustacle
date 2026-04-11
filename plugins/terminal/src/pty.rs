use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use rustacle_plugin_api::ModuleError;

/// A single PTY session (one tab = one session).
///
/// A background thread reads PTY output into a shared buffer.
/// `read()` drains the buffer instantly — never blocks the caller.
pub struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
    /// Output buffer filled by the background reader thread.
    output_buf: Arc<Mutex<Vec<u8>>>,
}

impl PtySession {
    /// Spawn a new PTY session with the user's default shell.
    ///
    /// Starts a background thread that reads PTY output into a buffer.
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

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| ModuleError::Internal(format!("failed to clone PTY reader: {e}")))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| ModuleError::Internal(format!("failed to take PTY writer: {e}")))?;

        tracing::info!(shell = %shell, cols, rows, "PTY session spawned");

        // Background reader thread: reads PTY output into shared buffer.
        let output_buf = Arc::new(Mutex::new(Vec::with_capacity(8192)));
        let buf_clone = Arc::clone(&output_buf);

        thread::Builder::new()
            .name("pty-reader".to_string())
            .spawn(move || {
                let mut tmp = [0u8; 4096];
                loop {
                    match reader.read(&mut tmp) {
                        Ok(n) if n > 0 => {
                            if let Ok(mut buf) = buf_clone.lock() {
                                buf.extend_from_slice(&tmp[..n]);
                            }
                        }
                        _ => break, // EOF or error
                    }
                }
            })
            .map_err(|e| ModuleError::Internal(format!("failed to spawn reader thread: {e}")))?;

        Ok(Self {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            child: Mutex::new(child),
            output_buf,
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
        let mut writer = self.writer.lock().expect("PTY writer lock poisoned");
        writer
            .write_all(data)
            .map_err(|e| ModuleError::Internal(format!("PTY write failed: {e}")))
    }

    /// Drain buffered output from the PTY. Returns immediately (non-blocking).
    ///
    /// # Panics
    /// Panics if the output buffer mutex is poisoned.
    #[must_use]
    pub fn read(&self) -> Vec<u8> {
        let mut buf = self.output_buf.lock().expect("output buffer lock poisoned");
        if buf.is_empty() {
            return Vec::new();
        }
        let data = buf.clone();
        buf.clear();
        data
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

    #[test]
    fn pty_write_and_read() {
        let session = PtySession::spawn(None, 80, 24).expect("PTY should spawn");

        // Give shell time to start
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Shell should have produced some output (prompt)
        let output = session.read();
        // Output may or may not be ready — just verify no panic
        assert!(output.len() >= 0); // always true, just exercises the path

        // Write and verify no error
        session.write(b"echo hello\r\n").expect("write should work");

        // Wait for response
        std::thread::sleep(std::time::Duration::from_millis(500));

        let output = session.read();
        // Should have captured some output from echo
        assert!(!output.is_empty(), "should have output after echo command");
    }
}
