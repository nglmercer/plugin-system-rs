use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};

#[derive(Serialize, Deserialize)]
pub(crate) struct ObsStatusResponse {
    connected: bool,
    host: String,
    port: u16,
    stream_active: bool,
    record_active: bool,
    record_paused: bool,
    virtual_cam_active: bool,
    replay_buffer_active: bool,
    current_scene: String,
    studio_mode: bool,
    cpu_usage: f64,
    memory_usage: f64,
    fps: f64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ObsSceneResponse {
    name: String,
    index: i32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ObsScenesListResponse {
    current_scene: String,
    scenes: Vec<ObsSceneResponse>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ObsInputResponse {
    name: String,
    kind: String,
    uuid: String,
    muted: bool,
    volume: f64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ObsTransitionResponse {
    name: String,
    kind: String,
    duration: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ObsSceneItemResponse {
    id: i32,
    name: String,
    enabled: bool,
}

#[derive(Deserialize)]
pub(crate) struct ObsConnectRequest {
    host: String,
    port: u16,
    password: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ObsSetSceneRequest {
    scene_name: String,
}

#[derive(Deserialize)]
pub(crate) struct ObsSetInputVolumeRequest {
    input_name: String,
    volume: f64,
}

#[derive(Deserialize)]
pub(crate) struct ObsSetInputMuteRequest {
    input_name: String,
    muted: bool,
}

#[derive(Deserialize)]
pub(crate) struct ObsSetTransitionRequest {
    name: String,
}

#[derive(Deserialize)]
pub(crate) struct ObsSetSceneItemRequest {
    scene_name: String,
    item_id: i32,
    enabled: bool,
}

#[derive(Deserialize)]
pub(crate) struct ObsSetStudioModeRequest {
    enabled: bool,
}

pub(crate) async fn get_obs_status(
    State(state): State<AppState>,
) -> Json<ApiResponse<ObsStatusResponse>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_response(
            &pm,
            "obs",
            "get_status",
            serde_json::json!({}),
        )
        .await,
    )
}

pub(crate) async fn connect_obs(
    State(state): State<AppState>,
    Json(req): Json<ObsConnectRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"host": req.host, "port": req.port, "password": req.password});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "connect",
            args,
            "Connected to OBS",
        )
        .await,
    )
}

pub(crate) async fn disconnect_obs(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "disconnect",
            serde_json::json!({}),
            "Disconnected from OBS",
        )
        .await,
    )
}

pub(crate) async fn start_stream(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "start_stream",
            serde_json::json!({}),
            "Stream started",
        )
        .await,
    )
}

pub(crate) async fn stop_stream(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "stop_stream",
            serde_json::json!({}),
            "Stream stopped",
        )
        .await,
    )
}

pub(crate) async fn start_record(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "start_record",
            serde_json::json!({}),
            "Recording started",
        )
        .await,
    )
}

pub(crate) async fn stop_record(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "stop_record",
            serde_json::json!({}),
            "Recording stopped",
        )
        .await,
    )
}

pub(crate) async fn toggle_record_pause(
    State(state): State<AppState>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "toggle_record_pause",
            serde_json::json!({}),
            "Record pause toggled",
        )
        .await,
    )
}

pub(crate) async fn get_scenes(
    State(state): State<AppState>,
) -> Json<ApiResponse<ObsScenesListResponse>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_response(
            &pm,
            "obs",
            "get_scenes",
            serde_json::json!({}),
        )
        .await,
    )
}

pub(crate) async fn set_current_scene(
    State(state): State<AppState>,
    Json(req): Json<ObsSetSceneRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"scene_name": req.scene_name});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "set_scene",
            args,
            "Scene changed",
        )
        .await,
    )
}

pub(crate) async fn get_inputs(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ObsInputResponse>>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_response(
            &pm,
            "obs",
            "get_inputs",
            serde_json::json!({}),
        )
        .await,
    )
}

pub(crate) async fn set_input_volume(
    State(state): State<AppState>,
    Json(req): Json<ObsSetInputVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"input_name": req.input_name, "volume": req.volume});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "set_input_volume",
            args,
            "Input volume set",
        )
        .await,
    )
}

pub(crate) async fn set_input_mute(
    State(state): State<AppState>,
    Json(req): Json<ObsSetInputMuteRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"input_name": req.input_name, "muted": req.muted});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "set_input_mute",
            args,
            "Input mute set",
        )
        .await,
    )
}

pub(crate) async fn toggle_virtual_cam(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "toggle_virtual_cam",
            serde_json::json!({}),
            "Virtual camera toggled",
        )
        .await,
    )
}

pub(crate) async fn save_replay(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "save_replay",
            serde_json::json!({}),
            "Replay saved",
        )
        .await,
    )
}

pub(crate) async fn get_transitions(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ObsTransitionResponse>>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_response(
            &pm,
            "obs",
            "get_transitions",
            serde_json::json!({}),
        )
        .await,
    )
}

pub(crate) async fn set_transition(
    State(state): State<AppState>,
    Json(req): Json<ObsSetTransitionRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"name": req.name});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "set_transition",
            args,
            "Transition set",
        )
        .await,
    )
}

pub(crate) async fn get_scene_items(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<Vec<ObsSceneItemResponse>>> {
    let scene_name = params.get("scene_name").cloned().unwrap_or_default();
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"scene_name": scene_name});
    Json(crate::api::helpers::call_plugin_response(&pm, "obs", "get_scene_items", args).await)
}

pub(crate) async fn set_scene_item_enabled(
    State(state): State<AppState>,
    Json(req): Json<ObsSetSceneItemRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"scene_name": req.scene_name, "item_id": req.item_id, "enabled": req.enabled});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "set_scene_item_enabled",
            args,
            "Scene item toggled",
        )
        .await,
    )
}

pub(crate) async fn get_studio_mode(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let pm = state.plugin_manager.plugin_manager();
    Json(
        crate::api::helpers::call_plugin_response(
            &pm,
            "obs",
            "get_studio_mode",
            serde_json::json!({}),
        )
        .await,
    )
}

pub(crate) async fn set_studio_mode(
    State(state): State<AppState>,
    Json(req): Json<ObsSetStudioModeRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let args = serde_json::json!({"enabled": req.enabled});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &pm,
            "obs",
            "set_studio_mode",
            args,
            "Studio mode set",
        )
        .await,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact payload `plugin-obs-wasm`'s `get_status` produces, including
    /// the two fields it adds beyond this crate's response type.
    ///
    /// This is the contract test the OBS boundary was missing. The plugin and
    /// `ObsStatusResponse` are declared in two crates that never see each
    /// other — one is compiled for `wasm32-wasip2` and outside the workspace —
    /// so nothing but a test like this notices when a field is renamed on one
    /// side.
    fn plugin_status_payload() -> serde_json::Value {
        serde_json::json!({
            "connected": true,
            "host": "127.0.0.1",
            "port": 4455,
            "stream_active": true,
            "record_active": false,
            "record_paused": false,
            "virtual_cam_active": false,
            "replay_buffer_active": true,
            "current_scene": "Escena",
            "studio_mode": false,
            "cpu_usage": 12.5,
            "memory_usage": 480.0,
            "fps": 60.0,
            // Extra fields the plugin sends and this crate does not model.
            "scenes": ["Escena", "Intro"],
            "last_error": null
        })
    }

    #[test]
    fn the_obs_plugin_payload_deserializes_with_every_field_populated() {
        let parsed: ObsStatusResponse =
            serde_json::from_value(plugin_status_payload()).expect("plugin payload should parse");

        // Asserting the values, not just that it parsed: a field that silently
        // fell back to a default would still "parse", and the widget would
        // show a plausible-looking zero.
        assert!(parsed.connected);
        assert_eq!(parsed.host, "127.0.0.1");
        assert_eq!(parsed.port, 4455);
        assert!(parsed.stream_active);
        assert!(!parsed.record_active);
        assert!(!parsed.record_paused);
        assert!(!parsed.virtual_cam_active);
        assert!(parsed.replay_buffer_active);
        assert_eq!(parsed.current_scene, "Escena");
        assert!(!parsed.studio_mode);
        assert_eq!(parsed.cpu_usage, 12.5);
        assert_eq!(parsed.memory_usage, 480.0);
        assert_eq!(parsed.fps, 60.0);
    }

    /// Fields the plugin adds must keep passing through harmlessly — that is
    /// what lets the plugin ship `scenes` without a lockstep release here.
    #[test]
    fn unknown_fields_from_the_plugin_are_ignored() {
        let mut payload = plugin_status_payload();
        payload["some_future_field"] = serde_json::json!("hello");
        assert!(serde_json::from_value::<ObsStatusResponse>(payload).is_ok());
    }

    /// The drift that must never be silent: a field renamed on the plugin side
    /// has to fail the deserialize so `call_plugin` logs a shape mismatch,
    /// rather than defaulting to zero and rendering a working-looking widget.
    #[test]
    fn a_renamed_field_fails_loudly_instead_of_defaulting() {
        for field in [
            "connected",
            "current_scene",
            "stream_active",
            "record_active",
            "fps",
            "cpu_usage",
        ] {
            let mut payload = plugin_status_payload();
            let value = payload[field].clone();
            payload.as_object_mut().unwrap().remove(field);
            payload[format!("{field}_renamed")] = value;

            assert!(
                serde_json::from_value::<ObsStatusResponse>(payload).is_err(),
                "renaming `{field}` must break deserialization, not default silently"
            );
        }
    }
}
