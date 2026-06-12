use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};

#[derive(Deserialize)]
pub(crate) struct SendHotkeyRequest {
    keys: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct SendHotkeyResponse {
    combo: String,
    keys: Vec<String>,
    simulated: bool,
    error: Option<String>,
}

pub(crate) async fn send_hotkey(
    State(state): State<AppState>,
    Json(req): Json<SendHotkeyRequest>,
) -> Json<ApiResponse<SendHotkeyResponse>> {
    let combo = req.keys.join("+");
    let keys_for_sim = req.keys.clone();

    let pm = state.plugin_manager.plugin_manager().clone();
    let sim_result = tokio::task::spawn_blocking(move || {
        let guard = pm.blocking_read();
        crate::api::helpers::call_plugin_ok_sync(&guard, "key-simulator", "simulate_keys", serde_json::json!({"keys": keys_for_sim}))
    })
    .await
    .unwrap_or(Err("Blocking task failed".to_string()));

    let (simulated, sim_error) = match sim_result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e)),
    };

    state.events.emit(sd_events::StreamEvent::ActionExecuted {
        action: sd_types::ActionId("hotkey".to_string()),
        result: sd_types::PluginResult::string(format!("Keys: {}", combo)),
    });

    if let Some(ref err) = sim_error {
        log::warn!("[Hotkey] simulation failed for {}: {}", combo, err);
    } else {
        log::info!("[Hotkey] simulation succeeded for {}", combo);
    }

    Json(ApiResponse::success(SendHotkeyResponse {
        combo,
        keys: req.keys,
        simulated,
        error: sim_error,
    }))
}

#[derive(Deserialize)]
pub(crate) struct RecordHotkeyRequest {
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
}

fn default_timeout() -> u64 {
    15000
}

#[derive(Serialize)]
pub(crate) struct RecordHotkeyResponse {
    combo: String,
}

pub(crate) async fn record_hotkey(
    State(state): State<AppState>,
    Json(req): Json<RecordHotkeyRequest>,
) -> Json<ApiResponse<RecordHotkeyResponse>> {
    let pm = state.plugin_manager.plugin_manager().clone();
    let result = tokio::task::spawn_blocking(move || {
        let guard = pm.blocking_read();
        crate::api::helpers::call_plugin_raw_sync(&guard, "key-simulator", "listen_for_combo", serde_json::json!({"timeout_ms": req.timeout_ms}))
            .ok_or("Key simulator plugin not available".to_string())
            .and_then(|result| {
                if let Some(combo) = result.get("combo").and_then(|v| v.as_str()) {
                    Ok(combo.to_string())
                } else {
                    Err(result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error")
                        .to_string())
                }
            })
    })
    .await
    .unwrap_or(Err("Recording failed".to_string()));

    match result {
        Ok(combo) => {
            log::info!("[Hotkey] Recorded: {}", combo);
            Json(ApiResponse::success(RecordHotkeyResponse { combo }))
        }
        Err(e) => Json(ApiResponse::error(e)),
    }
}

pub(crate) async fn reset_hotkey_recording(
    State(state): State<AppState>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager().clone();
    let guard = pm.blocking_read();
    let _ = crate::api::helpers::call_plugin_raw_sync(&guard, "key-simulator", "reset_recording", serde_json::json!({}));
    log::info!("[Hotkey] Recording state reset");
    Json(ApiResponse::success("Recording state reset".to_string()))
}
