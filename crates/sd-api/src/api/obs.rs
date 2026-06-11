use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};

#[derive(Serialize)]
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

#[derive(Serialize)]
pub(crate) struct ObsSceneResponse {
    name: String,
    index: i32,
}

#[derive(Serialize)]
pub(crate) struct ObsScenesListResponse {
    current_scene: String,
    scenes: Vec<ObsSceneResponse>,
}

#[derive(Serialize)]
pub(crate) struct ObsInputResponse {
    name: String,
    kind: String,
    uuid: String,
    muted: bool,
    volume: f64,
}

#[derive(Serialize)]
pub(crate) struct ObsTransitionResponse {
    name: String,
    kind: String,
    duration: u32,
}

#[derive(Serialize)]
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
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(data) = plugin.handle_command("get_status", args) {
                return Json(ApiResponse::success(ObsStatusResponse {
                    connected: data
                        .get("connected")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    host: data
                        .get("host")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    port: data.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                    stream_active: data
                        .get("stream_active")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    record_active: data
                        .get("record_active")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    record_paused: data
                        .get("record_paused")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    virtual_cam_active: data
                        .get("virtual_cam_active")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    replay_buffer_active: data
                        .get("replay_buffer_active")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    current_scene: data
                        .get("current_scene")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    studio_mode: data
                        .get("studio_mode")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    cpu_usage: data
                        .get("cpu_usage")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    memory_usage: data
                        .get("memory_usage")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    fps: data.get("fps").and_then(|v| v.as_f64()).unwrap_or(0.0),
                }));
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn connect_obs(
    State(state): State<AppState>,
    Json(req): Json<ObsConnectRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({
                "host": req.host,
                "port": req.port,
                "password": req.password
            });
            if let Some(result) = plugin.handle_command("connect", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Connected to OBS".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn disconnect_obs(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if plugin.handle_command("disconnect", args).is_some() {
                return Json(ApiResponse::success("Disconnected from OBS".to_string()));
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn start_stream(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(result) = plugin.handle_command("start_stream", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Stream started".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn stop_stream(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(result) = plugin.handle_command("stop_stream", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Stream stopped".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn start_record(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(result) = plugin.handle_command("start_record", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Recording started".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn stop_record(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(result) = plugin.handle_command("stop_record", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Recording stopped".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn toggle_record_pause(
    State(state): State<AppState>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(result) = plugin.handle_command("toggle_record_pause", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Record pause toggled".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn get_scenes(
    State(state): State<AppState>,
) -> Json<ApiResponse<ObsScenesListResponse>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(data) = plugin.handle_command("get_scenes", args) {
                let current_scene = data
                    .get("current_scene")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let scenes = data
                    .get("scenes")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|s| {
                                ObsSceneResponse {
                                    name: s
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    index: s.get("index").and_then(|v| v.as_i64()).unwrap_or(0)
                                        as i32,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Json(ApiResponse::success(ObsScenesListResponse {
                    current_scene,
                    scenes,
                }));
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn set_current_scene(
    State(state): State<AppState>,
    Json(req): Json<ObsSetSceneRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({"scene_name": req.scene_name});
            if let Some(result) = plugin.handle_command("set_scene", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Scene changed".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn get_inputs(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ObsInputResponse>>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(data) = plugin.handle_command("get_inputs", args) {
                let inputs = data
                    .get("inputs")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|i| {
                                ObsInputResponse {
                                    name: i
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    kind: i
                                        .get("kind")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    uuid: i
                                        .get("uuid")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    muted: i
                                        .get("muted")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    volume: i.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Json(ApiResponse::success(inputs));
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn set_input_volume(
    State(state): State<AppState>,
    Json(req): Json<ObsSetInputVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({"input_name": req.input_name, "volume": req.volume});
            if let Some(result) = plugin.handle_command("set_input_volume", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Input volume set".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn set_input_mute(
    State(state): State<AppState>,
    Json(req): Json<ObsSetInputMuteRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({"input_name": req.input_name, "muted": req.muted});
            if let Some(result) = plugin.handle_command("set_input_mute", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Input mute set".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn toggle_virtual_cam(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(result) = plugin.handle_command("toggle_virtual_cam", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Virtual camera toggled".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn save_replay(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(result) = plugin.handle_command("save_replay", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Replay saved".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn get_transitions(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ObsTransitionResponse>>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(data) = plugin.handle_command("get_transitions", args) {
                let transitions = data
                    .get("transitions")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|t| {
                                ObsTransitionResponse {
                                    name: t
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    kind: t
                                        .get("kind")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    duration: t
                                        .get("duration")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                        as u32,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Json(ApiResponse::success(transitions));
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn set_transition(
    State(state): State<AppState>,
    Json(req): Json<ObsSetTransitionRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({"name": req.name});
            if let Some(result) = plugin.handle_command("set_transition", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Transition set".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn get_scene_items(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<ApiResponse<Vec<ObsSceneItemResponse>>> {
    let scene_name = params.get("scene_name").cloned().unwrap_or_default();
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({"scene_name": scene_name});
            if let Some(data) = plugin.handle_command("get_scene_items", args) {
                let items = data
                    .get("items")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|i| {
                                ObsSceneItemResponse {
                                    id: i.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                                    name: i
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    enabled: i
                                        .get("enabled")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Json(ApiResponse::success(items));
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn set_scene_item_enabled(
    State(state): State<AppState>,
    Json(req): Json<ObsSetSceneItemRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({
                "scene_name": req.scene_name,
                "item_id": req.item_id,
                "enabled": req.enabled
            });
            if let Some(result) = plugin.handle_command("set_scene_item_enabled", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Scene item toggled".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn get_studio_mode(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({});
            if let Some(data) = plugin.handle_command("get_studio_mode", args) {
                let enabled = data
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                return Json(ApiResponse::success(enabled));
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}

pub(crate) async fn set_studio_mode(
    State(state): State<AppState>,
    Json(req): Json<ObsSetStudioModeRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("obs") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("ObsControl") {
            let args = serde_json::json!({"enabled": req.enabled});
            if let Some(result) = plugin.handle_command("set_studio_mode", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Studio mode set".to_string()));
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Json(ApiResponse::error(error.to_string()));
                }
            }
        }
    }

    Json(ApiResponse::error("OBS plugin not available"))
}
