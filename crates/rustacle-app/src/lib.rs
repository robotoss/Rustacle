use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use rustacle_ipc::commands::agent::{
    ListModelProfilesResponse, ProfileSummary, RespondPermissionRequest, SendPromptRequest,
    SendPromptResponse, StopTurnRequest, StopTurnResponse,
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
use rustacle_llm::provider::LlmProvider;
use rustacle_llm::types::{ChatMessage as LlmChatMessage, ChatRequest, Role as LlmRole};
use rustacle_llm_openai::OpenAiProvider;
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
    let message = request.message.clone();
    let session = Arc::clone(&state.agent_session);
    let settings = Arc::clone(&state.settings);

    // Try to load model profile from settings for real LLM call.
    let profile_name = request.model_profile.clone();

    tokio::spawn(async move {
        let start = std::time::Instant::now();

        // Give the frontend time to process START_TURN before emitting events.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        if cancel_token.is_cancelled() {
            emit_turn_end(&app, &turn_id_clone, &start);
            cleanup_session(&session, &turn_id_clone, &message, "Cancelled").await;
            return;
        }

        // Try to find the model profile and call the real LLM.
        let answer_text = match run_llm_turn(
            &app,
            &turn_id_clone,
            &message,
            profile_name.as_deref(),
            &settings,
            &cancel_token,
        )
        .await
        {
            Ok(text) => text,
            Err(err_msg) => {
                // LLM failed — emit the error as a thought, then a placeholder answer.
                emit_reasoning(
                    &app,
                    &turn_id_clone,
                    IpcReasoningStep::Error {
                        message: err_msg.clone(),
                        retryable: false,
                    },
                );
                format!("LLM error: {err_msg}")
            }
        };

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

/// Run a real LLM turn: load profile from settings, build request, stream response.
/// Returns the full answer text on success, or an error message string on failure.
#[allow(clippy::too_many_lines)]
async fn run_llm_turn(
    app: &tauri::AppHandle,
    turn_id: &str,
    message: &str,
    profile_name: Option<&str>,
    settings: &rustacle_settings::SettingsStore,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<String, String> {
    use futures_util::StreamExt;

    // Load model profiles from settings.
    let profiles_json: Vec<serde_json::Value> = settings
        .get(SettingKey::ModelProfiles)
        .map_err(|e| format!("Failed to load profiles: {e}"))?;

    let name = profile_name.unwrap_or("default");

    tracing::info!(
        profile = name,
        count = profiles_json.len(),
        "loading model profile"
    );

    let profile = profiles_json
        .iter()
        .find(|p| p.get("name").and_then(serde_json::Value::as_str) == Some(name))
        .or_else(|| profiles_json.first())
        .ok_or_else(|| {
            "No model profiles configured. Go to Settings -> Model Profiles to add one.".to_owned()
        })?;

    let provider_str = profile
        .get("provider")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("openai");
    let model = profile
        .get("model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_owned();
    let api_base = profile
        .get("api_base")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_owned();
    let api_key = profile
        .get("api_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_owned();
    #[allow(clippy::cast_possible_truncation)]
    let max_tokens = profile
        .get("max_tokens")
        .and_then(serde_json::Value::as_u64)
        .map(|v| v as u32);
    #[allow(clippy::cast_possible_truncation)]
    let temperature = profile
        .get("temperature")
        .and_then(serde_json::Value::as_f64)
        .map(|v| v as f32);

    if model.is_empty() {
        return Err("Model name is empty in profile. Edit the profile in Settings.".to_owned());
    }

    // Determine API base — use defaults for known providers.
    let effective_base = if api_base.is_empty() {
        match provider_str {
            "openai" => "https://api.openai.com/v1".to_owned(),
            "anthropic" => "https://api.anthropic.com/v1".to_owned(),
            _ => {
                return Err(
                    "No API base URL set. For local models, set the endpoint URL.".to_owned(),
                );
            }
        }
    } else {
        api_base
    };

    let key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };

    tracing::info!(
        provider = provider_str,
        llm_model = %model,
        base = %effective_base,
        has_key = key.is_some(),
        "connecting to LLM"
    );

    // Build the provider (OpenAI-compatible for all providers for now).
    let llm = OpenAiProvider::new(effective_base, key);

    let chat_request = ChatRequest {
        model,
        messages: vec![
            LlmChatMessage {
                role: LlmRole::System,
                content: "You are Rustacle, a helpful assistant. Be concise.".to_owned(),
                tool_call_id: None,
                name: None,
            },
            LlmChatMessage {
                role: LlmRole::User,
                content: message.to_owned(),
                tool_call_id: None,
                name: None,
            },
        ],
        tools: Vec::new(),
        max_tokens,
        temperature,
    };

    emit_reasoning(
        app,
        turn_id,
        IpcReasoningStep::Thought {
            text: "Connecting to LLM...".to_owned(),
            partial: true,
        },
    );

    // Stream the response — cancellable.
    let stream = tokio::select! {
        r = llm.stream(chat_request, cancel.clone()) => {
            match r {
                Ok(s) => {
                    tracing::info!("LLM stream connected");
                    s
                }
                Err(e) => {
                    tracing::error!(error = %e, "LLM stream failed");
                    return Err(format!("LLM connection failed: {e}"));
                }
            }
        }
        () = cancel.cancelled() => {
            return Err("Cancelled".to_owned());
        }
    };

    tokio::pin!(stream);

    let mut full_text = String::new();
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;

    loop {
        let delta = tokio::select! {
            d = stream.next() => d,
            () = cancel.cancelled() => {
                if !full_text.is_empty() {
                    emit_reasoning(app, turn_id, IpcReasoningStep::Answer {
                        text: full_text.clone(),
                    });
                }
                return Ok(full_text);
            }
        };

        let Some(delta_result) = delta else { break };

        match delta_result {
            Ok(rustacle_llm::ChatDelta::Text { text }) => {
                full_text.push_str(&text);
                emit_reasoning(
                    app,
                    turn_id,
                    IpcReasoningStep::Thought {
                        text,
                        partial: true,
                    },
                );
            }
            Ok(rustacle_llm::ChatDelta::Usage {
                input_tokens: i,
                output_tokens: o,
            }) => {
                input_tokens = i;
                output_tokens = o;
                let _ = app.emit(
                    "agent:cost",
                    &CostSampleEvent {
                        turn_id: turn_id.to_owned(),
                        input_tokens,
                        output_tokens,
                    },
                );
            }
            Ok(rustacle_llm::ChatDelta::Done)
            | Err(rustacle_llm::provider::LlmError::Cancelled) => break,
            Ok(_) => {} // ToolUseStart etc — ignore for now
            Err(e) => return Err(format!("Stream error: {e}")),
        }
    }

    tracing::info!(
        chars = full_text.len(),
        input_tokens,
        output_tokens,
        "LLM turn complete"
    );

    // Emit the final answer.
    if full_text.is_empty() {
        "(Empty response from LLM)".clone_into(&mut full_text);
    }

    emit_reasoning(
        app,
        turn_id,
        IpcReasoningStep::Answer {
            text: full_text.clone(),
        },
    );

    let _ = app.emit(
        "agent:cost",
        &CostSampleEvent {
            turn_id: turn_id.to_owned(),
            input_tokens,
            output_tokens,
        },
    );

    Ok(full_text)
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
