use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use rustacle_ipc::commands::plugins::{
    ListPluginsResponse, PluginCallRequest, PluginCallResponse, PluginState, PluginSummary,
};
use rustacle_ipc::commands::system::PingResponse;
use rustacle_ipc::errors::RustacleError;
use rustacle_kernel::demo_plugin::DemoPlugin;
use rustacle_kernel::{AppState, Kernel, PluginRegistry, lifecycle};
use rustacle_plugin_api::RustacleModule;
use rustacle_plugin_terminal::TerminalPlugin;

/// Ping command — proves direct IPC round-trip works.
#[allow(clippy::unnecessary_wraps)]
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

/// List all loaded plugins.
#[tauri::command]
#[specta::specta]
async fn list_plugins(
    state: tauri::State<'_, AppState>,
) -> Result<ListPluginsResponse, RustacleError> {
    let ids = state.registry.list_ids().await;
    let plugins = ids
        .into_iter()
        .map(|id| PluginSummary {
            id,
            version: "0.1.0".to_string(),
            state: PluginState::Running,
        })
        .collect();
    Ok(ListPluginsResponse { plugins })
}

/// Call a plugin command through the kernel registry.
/// This is the core integration proof: UI → IPC → Kernel → Plugin → back.
#[tauri::command]
#[specta::specta]
async fn plugin_call(
    state: tauri::State<'_, AppState>,
    request: PluginCallRequest,
) -> Result<PluginCallResponse, RustacleError> {
    let payload = Bytes::from(request.payload.clone());

    let result = state
        .registry
        .call(&request.plugin_id, &request.command, payload)
        .await
        .map_err(|e| RustacleError::PluginError {
            plugin_id: request.plugin_id.clone(),
            message: e.to_string(),
        })?;

    let data = String::from_utf8(result.to_vec()).map_err(|e| RustacleError::Internal {
        message: format!("plugin returned invalid UTF-8: {e}"),
    })?;

    Ok(PluginCallResponse {
        plugin_id: request.plugin_id,
        data,
    })
}

/// Build the tauri-specta builder with all commands.
#[must_use]
pub fn specta_builder() -> tauri_specta::Builder {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        ping,
        version,
        list_plugins,
        plugin_call,
    ])
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

    let (kernel, registry) = rt.block_on(async {
        let mut kernel = Kernel::new();
        kernel.start().await.expect("kernel start failed");

        let registry = PluginRegistry::new();

        // Register built-in plugins.
        let mut demo = DemoPlugin::new();
        demo.init().await.expect("demo plugin init failed");
        registry.register(Box::new(demo)).await;

        let mut terminal = TerminalPlugin::new();
        terminal.init().await.expect("terminal plugin init failed");
        registry.register(Box::new(terminal)).await;

        (Arc::new(kernel), Arc::new(registry))
    });

    let app_state = AppState {
        kernel: Arc::clone(&kernel),
        registry: Arc::clone(&registry),
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
