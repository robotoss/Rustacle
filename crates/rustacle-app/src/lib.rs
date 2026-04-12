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
use rustacle_ipc::commands::settings::{
    GetSettingRequest, GetSettingResponse, SetSettingRequest, TestModelRequest, TestModelResponse,
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
use rustacle_settings::SettingKey;
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

    // Spawn the agent turn in the background.
    // Placeholder: emits thought + answer. Real harness replaces this.
    tokio::spawn(async move {
        let start = std::time::Instant::now();

        // Give the frontend time to process START_TURN before emitting events.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Check cancellation before each step.
        if cancel_token.is_cancelled() {
            emit_turn_end(&app, &turn_id_clone, &start);
            cleanup_session(&session, &turn_id_clone, &message, "Cancelled").await;
            return;
        }

        let mode_label = match mode {
            AgentMode::Chat => "Chat",
            AgentMode::Plan => "Plan",
            AgentMode::Ask => "Ask",
        };

        emit_reasoning(
            &app,
            &turn_id_clone,
            IpcReasoningStep::Thought {
                text: format!("[{mode_label} mode] Processing: {message}"),
                partial: false,
            },
        );

        // Simulate LLM processing time — cancellable.
        tokio::select! {
            () = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
            () = cancel_token.cancelled() => {
                emit_reasoning(&app, &turn_id_clone, IpcReasoningStep::Answer {
                    text: "Cancelled by user.".to_owned(),
                });
                emit_turn_end(&app, &turn_id_clone, &start);
                cleanup_session(&session, &turn_id_clone, &message, "Cancelled").await;
                return;
            }
        }

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
                 Configure an LLM provider in Settings -> Model Profiles to enable real agent interaction."
            ),
        };

        emit_reasoning(
            &app,
            &turn_id_clone,
            IpcReasoningStep::Answer {
                text: answer_text.clone(),
            },
        );

        let _ = app.emit(
            "agent:cost",
            &CostSampleEvent {
                turn_id: turn_id_clone.clone(),
                input_tokens: 0,
                output_tokens: 0,
            },
        );

        emit_turn_end(&app, &turn_id_clone, &start);
        cleanup_session(&session, &turn_id_clone, &message, &answer_text).await;
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

/// List available model profiles from settings store.
#[tauri::command]
#[specta::specta]
async fn list_model_profiles(
    state: tauri::State<'_, AppState>,
) -> Result<ListModelProfilesResponse, RustacleError> {
    let profiles_json: Vec<serde_json::Value> = state
        .settings
        .get(SettingKey::ModelProfiles)
        .map_err(|e| RustacleError::Internal {
            message: e.to_string(),
        })?;

    let profiles = profiles_json
        .iter()
        .filter_map(|v| {
            Some(ProfileSummary {
                name: v.get("name")?.as_str()?.to_owned(),
                provider: v.get("provider")?.as_str()?.to_owned(),
                model: v.get("model")?.as_str()?.to_owned(),
            })
        })
        .collect();

    Ok(ListModelProfilesResponse { profiles })
}

/// Get a setting value by key.
#[tauri::command]
#[specta::specta]
async fn get_setting(
    state: tauri::State<'_, AppState>,
    request: GetSettingRequest,
) -> Result<GetSettingResponse, RustacleError> {
    let key = SettingKey::from_key_str(&request.key).ok_or_else(|| RustacleError::NotFound {
        resource: format!("setting key: {}", request.key),
    })?;

    let value = state
        .settings
        .get_json(key)
        .map_err(|e| RustacleError::Internal {
            message: e.to_string(),
        })?;

    let value_json = serde_json::to_string(&value).map_err(|e| RustacleError::Internal {
        message: e.to_string(),
    })?;

    Ok(GetSettingResponse {
        key: request.key,
        value_json,
    })
}

/// Set a setting value by key. Emits `settings:changed` event on success.
#[tauri::command]
#[specta::specta]
async fn set_setting(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: SetSettingRequest,
) -> Result<(), RustacleError> {
    let key = SettingKey::from_key_str(&request.key).ok_or_else(|| RustacleError::NotFound {
        resource: format!("setting key: {}", request.key),
    })?;

    let value: serde_json::Value =
        serde_json::from_str(&request.value_json).map_err(|e| RustacleError::InvalidInput {
            field: "value_json".to_owned(),
            message: e.to_string(),
        })?;

    state
        .settings
        .set_json(key, value)
        .map_err(|e| RustacleError::Internal {
            message: e.to_string(),
        })?;

    // Notify all frontend subscribers that a setting changed.
    let _ = app.emit(
        "settings:changed",
        &serde_json::json!({ "key": request.key }),
    );

    Ok(())
}

/// Test a model connection by sending a minimal chat completion request.
#[tauri::command]
#[specta::specta]
async fn test_model_connection(
    request: TestModelRequest,
) -> Result<TestModelResponse, RustacleError> {
    let start = std::time::Instant::now();

    let base = if request.api_base.is_empty() {
        match request.provider.as_str() {
            "openai" => "https://api.openai.com/v1".to_owned(),
            "anthropic" => "https://api.anthropic.com".to_owned(),
            _ => {
                return Ok(TestModelResponse {
                    ok: false,
                    message: "No API base URL provided".to_owned(),
                    latency_ms: 0,
                });
            }
        }
    } else {
        request.api_base.trim_end_matches('/').to_owned()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| RustacleError::Internal {
            message: e.to_string(),
        })?;

    // Build request based on provider
    let result = if request.provider == "anthropic" {
        client
            .post(format!("{base}/v1/messages"))
            .header("x-api-key", &request.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": request.model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .send()
            .await
    } else {
        // OpenAI-compatible (works for local too)
        let mut req = client
            .post(format!("{base}/chat/completions"))
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": request.model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }));

        if !request.api_key.is_empty() {
            req = req.header("authorization", format!("Bearer {}", request.api_key));
        }

        req.send().await
    };

    #[allow(clippy::cast_possible_truncation)]
    let latency_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                Ok(TestModelResponse {
                    ok: true,
                    message: format!("Connected ({status})"),
                    latency_ms,
                })
            } else {
                let body = resp.text().await.unwrap_or_default();
                let msg = if body.len() > 200 {
                    format!("{status}: {}...", &body[..200])
                } else {
                    format!("{status}: {body}")
                };
                Ok(TestModelResponse {
                    ok: false,
                    message: msg,
                    latency_ms,
                })
            }
        }
        Err(e) => Ok(TestModelResponse {
            ok: false,
            message: e.to_string(),
            latency_ms,
        }),
    }
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

/// Emit a reasoning step event.
fn emit_reasoning(app: &tauri::AppHandle, turn_id: &str, step: IpcReasoningStep) {
    let event = ReasoningStepEvent {
        id: ulid::Ulid::new().to_string(),
        parent_id: None,
        turn_id: turn_id.to_owned(),
        ts_ms: now_ms(),
        step,
    };
    let _ = app.emit("agent:reasoning", &event);
}

/// Emit a `turn_end` event.
fn emit_turn_end(app: &tauri::AppHandle, turn_id: &str, start: &std::time::Instant) {
    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let _ = app.emit(
        "agent:turn_end",
        &TurnEndEvent {
            turn_id: turn_id.to_owned(),
            duration_ms,
            input_tokens: 0,
            output_tokens: 0,
            tool_calls: 0,
        },
    );
}

/// Cleanup session after a turn completes.
async fn cleanup_session(
    session: &Arc<Mutex<rustacle_kernel::AgentSession>>,
    turn_id: &str,
    user_message: &str,
    answer: &str,
) {
    let mut sess = session.lock().await;
    sess.active_cancels.remove(turn_id);
    sess.history
        .push(rustacle_kernel::state::AgentHistoryEntry {
            user_message: user_message.to_owned(),
            assistant_answer: answer.to_owned(),
        });
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
        get_setting,
        set_setting,
        test_model_connection,
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

    // Open settings database in the app data directory.
    let settings_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("rustacle")
        .join("settings.db");
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create settings directory");
    }
    let settings =
        rustacle_settings::SettingsStore::open(&settings_path).expect("failed to open settings");

    let app_state = AppState {
        kernel: Arc::clone(&kernel),
        registry: Arc::clone(&registry),
        agent_session: Arc::new(Mutex::new(AgentSession::default())),
        settings: Arc::new(settings),
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
