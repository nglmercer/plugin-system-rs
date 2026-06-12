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
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_response(&manager, "obs", "get_status", serde_json::json!({})).await)
}

pub(crate) async fn connect_obs(
    State(state): State<AppState>,
    Json(req): Json<ObsConnectRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"host": req.host, "port": req.port, "password": req.password});
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "connect", args, "Connected to OBS").await)
}

pub(crate) async fn disconnect_obs(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "disconnect", serde_json::json!({}), "Disconnected from OBS").await)
}

pub(crate) async fn start_stream(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "start_stream", serde_json::json!({}), "Stream started").await)
}

pub(crate) async fn stop_stream(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "stop_stream", serde_json::json!({}), "Stream stopped").await)
}

pub(crate) async fn start_record(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "start_record", serde_json::json!({}), "Recording started").await)
}

pub(crate) async fn stop_record(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "stop_record", serde_json::json!({}), "Recording stopped").await)
}

pub(crate) async fn toggle_record_pause(
    State(state): State<AppState>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "toggle_record_pause", serde_json::json!({}), "Record pause toggled").await)
}

pub(crate) async fn get_scenes(
    State(state): State<AppState>,
) -> Json<ApiResponse<ObsScenesListResponse>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_response(&manager, "obs", "get_scenes", serde_json::json!({})).await)
}

pub(crate) async fn set_current_scene(
    State(state): State<AppState>,
    Json(req): Json<ObsSetSceneRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"scene_name": req.scene_name});
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "set_scene", args, "Scene changed").await)
}

pub(crate) async fn get_inputs(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ObsInputResponse>>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_response(&manager, "obs", "get_inputs", serde_json::json!({})).await)
}

pub(crate) async fn set_input_volume(
    State(state): State<AppState>,
    Json(req): Json<ObsSetInputVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"input_name": req.input_name, "volume": req.volume});
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "set_input_volume", args, "Input volume set").await)
}

pub(crate) async fn set_input_mute(
    State(state): State<AppState>,
    Json(req): Json<ObsSetInputMuteRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"input_name": req.input_name, "muted": req.muted});
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "set_input_mute", args, "Input mute set").await)
}

pub(crate) async fn toggle_virtual_cam(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "toggle_virtual_cam", serde_json::json!({}), "Virtual camera toggled").await)
}

pub(crate) async fn save_replay(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "save_replay", serde_json::json!({}), "Replay saved").await)
}

pub(crate) async fn get_transitions(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ObsTransitionResponse>>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_response(&manager, "obs", "get_transitions", serde_json::json!({})).await)
}

pub(crate) async fn set_transition(
    State(state): State<AppState>,
    Json(req): Json<ObsSetTransitionRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"name": req.name});
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "set_transition", args, "Transition set").await)
}

pub(crate) async fn get_scene_items(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<Vec<ObsSceneItemResponse>>> {
    let scene_name = params.get("scene_name").cloned().unwrap_or_default();
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"scene_name": scene_name});
    Json(crate::api::helpers::call_plugin_response(&manager, "obs", "get_scene_items", args).await)
}

pub(crate) async fn set_scene_item_enabled(
    State(state): State<AppState>,
    Json(req): Json<ObsSetSceneItemRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"scene_name": req.scene_name, "item_id": req.item_id, "enabled": req.enabled});
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "set_scene_item_enabled", args, "Scene item toggled").await)
}

pub(crate) async fn get_studio_mode(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    Json(crate::api::helpers::call_plugin_response(&manager, "obs", "get_studio_mode", serde_json::json!({})).await)
}

pub(crate) async fn set_studio_mode(
    State(state): State<AppState>,
    Json(req): Json<ObsSetStudioModeRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"enabled": req.enabled});
    Json(crate::api::helpers::call_plugin_ok_response(&manager, "obs", "set_studio_mode", args, "Studio mode set").await)
}
