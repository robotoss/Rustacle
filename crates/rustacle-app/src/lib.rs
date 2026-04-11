use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rustacle_ipc::commands::system::PingResponse;
use rustacle_ipc::errors::RustacleError;
use rustacle_kernel::{AppState, Kernel, lifecycle};

/// Ping command — proves IPC round-trip works.
#[allow(clippy::unnecessary_wraps)] // Result required by tauri-specta for typed errors
#[tauri::command]
#[specta::specta]
fn ping() -> Result<PingResponse, RustacleError> {
    #[allow(clippy::cast_possible_truncation)]
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);

    Ok(PingResponse {
        message: "pong".to_string(),
        timestamp,
    })
}

/// Version command — returns the app version from Cargo.toml.
#[tauri::command]
#[specta::specta]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Build the tauri-specta builder with all commands.
#[must_use]
pub fn specta_builder() -> tauri_specta::Builder {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![ping, version])
}

/// Run the Rustacle application.
///
/// # Panics
/// Panics if the tokio runtime, kernel startup, or Tauri app fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    lifecycle::init_tracing();

    let builder = specta_builder();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    let kernel = rt.block_on(async {
        let mut kernel = Kernel::new();
        kernel.start().await.expect("kernel start failed");
        Arc::new(kernel)
    });

    let app_state = AppState {
        kernel: Arc::clone(&kernel),
    };

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .manage(app_state)
        .run(tauri::generate_context!())
        .expect("error while running Rustacle");
}
