use axum::{extract::State, Json};
use plugin_key_simulator::KeySimulator;
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

    let sim_result = tokio::task::spawn_blocking(move || {
        let sim = plugin_key_simulator::KeySimulatorPlugin::new();
        sim.simulate_keys(&keys_for_sim)
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
    Json(req): Json<RecordHotkeyRequest>,
) -> Json<ApiResponse<RecordHotkeyResponse>> {
    let result = tokio::task::spawn_blocking(move || {
        plugin_key_simulator::KeySimulatorPlugin::listen_for_combo(req.timeout_ms)
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

pub(crate) async fn reset_hotkey_recording() -> Json<ApiResponse<String>> {
    plugin_key_simulator::KeySimulatorPlugin::reset_recording_state();
    log::info!("[Hotkey] Recording state reset");
    Json(ApiResponse::success("Recording state reset".to_string()))
}
