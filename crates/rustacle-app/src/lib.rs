use std::sync::Arc;

use rustacle_kernel::{AppState, Kernel, lifecycle};

/// Run the Rustacle application.
///
/// # Panics
/// Panics if the tokio runtime, kernel startup, or Tauri app fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    lifecycle::init_tracing();

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
        .manage(app_state)
        .run(tauri::generate_context!())
        .expect("error while running Rustacle");
}
