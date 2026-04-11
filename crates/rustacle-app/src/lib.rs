use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use rustacle_ipc::commands::agent::{
    AgentMode, ListModelProfilesResponse, ProfileSummary, RespondPermissionRequest,
    SendPromptRequest, SendPromptResponse, StopTurnRequest, StopTurnResponse,
};
use rustacle_ipc::commands::plugins::{
    ListPluginsResponse, PluginCallRequest, PluginCallResponse, PluginState, PluginSummary,
};
use rustacle_ipc::commands::system::PingResponse;
use rustacle_ipc::errors::RustacleError;
use rustacle_ipc::events::agent::{
    CostSampleEvent, ReasoningStep as IpcReasoningStep, ReasoningStepEvent, TurnEndEvent,
};
use rustacle_kernel::demo_plugin::DemoPlugin;
use rustacle_kernel::{AgentSession, AppState, Kernel, PluginRegistry, lifecycle};
use rustacle_plugin_api::RustacleModule;
use rustacle_plugin_terminal::TerminalPlugin;
use tauri::Emitter;
use tokio::sync::Mutex;
use tracing::warn;

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

/// Send a prompt to the agent, starting a new turn.
///
/// Spawns the harness loop in the background and streams reasoning steps
/// as Tauri events. Returns the turn ID immediately.
#[tauri::command]
#[specta::specta]
async fn send_prompt(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: SendPromptRequest,
) -> Result<SendPromptResponse, RustacleError> {
    let turn_id = ulid::Ulid::new().to_string();
    let cancel_token = tokio_util::sync::CancellationToken::new();

    // Store cancel token
    {
        let mut session = state.agent_session.lock().await;
        session
            .active_cancels
            .insert(turn_id.clone(), cancel_token.clone());
    }

    let turn_id_clone = turn_id.clone();
    let mode = request.mode;
    let message = request.message.clone();
    let session = Arc::clone(&state.agent_session);

    // Spawn the agent turn in the background
    tokio::spawn(async move {
        // For now, emit a placeholder thought + answer since we don't have
        // a configured LLM provider wired in yet. The real harness integration
        // will replace this once the LLM registry is available in AppState.
        let start = std::time::Instant::now();

        let mode_label = match mode {
            AgentMode::Chat => "Chat",
            AgentMode::Plan => "Plan",
            AgentMode::Ask => "Ask",
        };

        // Emit a thought showing the mode
        let thought_step = ReasoningStepEvent {
            id: ulid::Ulid::new().to_string(),
            parent_id: None,
            turn_id: turn_id_clone.clone(),
            ts_ms: now_ms(),
            step: IpcReasoningStep::Thought {
                text: format!("[{mode_label} mode] Processing: {message}"),
                partial: false,
            },
        };
        let _ = app.emit("agent:reasoning", &thought_step);

        // Simulate a brief delay for responsiveness
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Emit the answer
        let answer_text = match mode {
            AgentMode::Ask => format!(
                "This is the Ask mode placeholder response to: \"{message}\"\n\n\
                 Once an LLM provider is configured in Settings, this will be replaced \
                 with a real response."
            ),
            AgentMode::Plan => format!(
                "**Plan mode** — here's a placeholder plan for: \"{message}\"\n\n\
                 1. Analyze the request\n2. Identify affected files\n3. Propose changes\n\n\
                 _Configure an LLM provider in Settings to get real plans._"
            ),
            AgentMode::Chat => format!(
                "Placeholder response to: \"{message}\"\n\n\
                 Configure an LLM provider in Settings → Model Profiles to enable real agent interaction."
            ),
        };

        let answer_step = ReasoningStepEvent {
            id: ulid::Ulid::new().to_string(),
            parent_id: None,
            turn_id: turn_id_clone.clone(),
            ts_ms: now_ms(),
            step: IpcReasoningStep::Answer {
                text: answer_text.clone(),
            },
        };
        let _ = app.emit("agent:reasoning", &answer_step);

        // Emit cost
        let cost = CostSampleEvent {
            turn_id: turn_id_clone.clone(),
            input_tokens: 0,
            output_tokens: 0,
        };
        let _ = app.emit("agent:cost", &cost);

        // Emit turn end
        #[allow(clippy::cast_possible_truncation)]
        let duration_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        let turn_end = TurnEndEvent {
            turn_id: turn_id_clone.clone(),
            duration_ms,
            input_tokens: 0,
            output_tokens: 0,
            tool_calls: 0,
        };
        let _ = app.emit("agent:turn_end", &turn_end);

        // Update session history
        {
            let mut sess = session.lock().await;
            sess.active_cancels.remove(&turn_id_clone);
            sess.history
                .push(rustacle_kernel::state::AgentHistoryEntry {
                    user_message: message,
                    assistant_answer: answer_text,
                });
        }
    });

    Ok(SendPromptResponse { turn_id })
}

/// Stop an active agent turn.
#[tauri::command]
#[specta::specta]
async fn stop_turn(
    state: tauri::State<'_, AppState>,
    request: StopTurnRequest,
) -> Result<StopTurnResponse, RustacleError> {
    let session = state.agent_session.lock().await;
    if let Some(token) = session.active_cancels.get(&request.turn_id) {
        token.cancel();
        Ok(StopTurnResponse { cancelled: true })
    } else {
        Ok(StopTurnResponse { cancelled: false })
    }
}

/// List available model profiles from settings.
#[allow(clippy::unnecessary_wraps)]
#[tauri::command]
#[specta::specta]
async fn list_model_profiles() -> Result<ListModelProfilesResponse, RustacleError> {
    // TODO: Read from SettingsStore once it's wired into AppState.
    // For now return an empty list — the UI will show "No profiles" placeholder.
    Ok(ListModelProfilesResponse {
        profiles: vec![ProfileSummary {
            name: "default".to_string(),
            provider: "none".to_string(),
            model: "not configured".to_string(),
        }],
    })
}

/// Respond to a permission request from the agent.
#[tauri::command]
#[specta::specta]
async fn respond_permission(_request: RespondPermissionRequest) -> Result<(), RustacleError> {
    // TODO: Route decision to the harness via oneshot channel
    // once permission flow is fully wired.
    warn!("respond_permission not yet connected to harness");
    Ok(())
}

/// Get current Unix time in milliseconds.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() * 1000 + u64::from(d.subsec_millis()))
}

/// Build the tauri-specta builder with all commands.
#[must_use]
pub fn specta_builder() -> tauri_specta::Builder {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        ping,
        version,
        list_plugins,
        plugin_call,
        send_prompt,
        stop_turn,
        list_model_profiles,
        respond_permission,
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
        agent_session: Arc::new(Mutex::new(AgentSession::default())),
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
