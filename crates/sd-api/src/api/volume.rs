use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};

#[derive(Serialize, Deserialize)]
pub(crate) struct VolumeStateResponse {
    master_volume: f32,
    muted: bool,
    default_device_name: String,
    platform_supported: bool,
    per_app_supported: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct AppVolumeResponse {
    name: String,
    volume: f32,
    muted: bool,
    pid: Option<u32>,
}

#[derive(Serialize)]
pub(crate) struct VolumeDataResponse {
    state: VolumeStateResponse,
    apps: Vec<AppVolumeResponse>,
}

#[derive(Deserialize)]
pub(crate) struct SetVolumeRequest {
    volume: f32,
}

#[derive(Deserialize)]
pub(crate) struct SetMuteRequest {
    muted: bool,
}

#[derive(Deserialize)]
pub(crate) struct SetAppVolumeRequest {
    app_name: String,
    volume: f32,
}

#[derive(Deserialize)]
pub(crate) struct SetAppMuteRequest {
    app_name: String,
    muted: bool,
}

fn parse_volume_data(data: serde_json::Value) -> Option<VolumeDataResponse> {
    let state = data.get("state")?;
    let apps = data
        .get("apps")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    Some(VolumeDataResponse {
        state: VolumeStateResponse {
            master_volume: state
                .get("master_volume")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            muted: state
                .get("muted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            default_device_name: state
                .get("default_device_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            platform_supported: data
                .get("platform_supported")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            per_app_supported: data
                .get("per_app_supported")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        apps: apps
            .into_iter()
            .map(|a| {
                AppVolumeResponse {
                    name: a
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    volume: a.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    muted: a.get("muted").and_then(|v| v.as_bool()).unwrap_or(false),
                    pid: a.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
                }
            })
            .collect(),
    })
}

pub(crate) async fn get_volume_state(
    State(state): State<AppState>,
) -> Json<ApiResponse<VolumeDataResponse>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let plugin = plugin_arc.read().expect("plugin lock poisoned");
        if plugin.has_interface("VolumeMaster") {
            if let Some(data) = plugin.interface_data() {
                if let Some(resp) = parse_volume_data(data) {
                    return Json(ApiResponse::success(resp));
                }
            }
        }
    }

    Json(ApiResponse::error("Volume plugin not available"))
}

pub(crate) async fn get_app_volumes(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<AppVolumeResponse>>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let plugin = plugin_arc.read().expect("plugin lock poisoned");
        if plugin.has_interface("VolumeMaster") {
            if let Some(data) = plugin.interface_data() {
                let apps = data
                    .get("apps")
                    .and_then(|a| a.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|a| {
                                AppVolumeResponse {
                                    name: a
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    volume: a.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0)
                                        as f32,
                                    muted: a
                                        .get("muted")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    pid: a.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Json(ApiResponse::success(apps));
            }
        }
    }

    Json(ApiResponse::error("Volume plugin not available"))
}

pub(crate) async fn set_master_volume(
    State(state): State<AppState>,
    Json(req): Json<SetVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("VolumeMaster") {
            let args = serde_json::json!({"volume": req.volume});
            if let Some(result) = plugin.handle_command("set_volume", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Volume set".to_string()));
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

    Json(ApiResponse::error("Volume plugin not available"))
}

pub(crate) async fn set_master_mute(
    State(state): State<AppState>,
    Json(req): Json<SetMuteRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("VolumeMaster") {
            let args = serde_json::json!({"muted": req.muted});
            if let Some(result) = plugin.handle_command("set_mute", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("Mute set".to_string()));
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

    Json(ApiResponse::error("Volume plugin not available"))
}

pub(crate) async fn set_app_volume(
    State(state): State<AppState>,
    Json(req): Json<SetAppVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("VolumeMaster") {
            let args = serde_json::json!({"app_name": req.app_name, "volume": req.volume});
            if let Some(result) = plugin.handle_command("set_app_volume", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("App volume set".to_string()));
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

    Json(ApiResponse::error("Volume plugin not available"))
}

pub(crate) async fn set_app_mute(
    State(state): State<AppState>,
    Json(req): Json<SetAppMuteRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        if plugin.has_interface("VolumeMaster") {
            let args = serde_json::json!({"app_name": req.app_name, "muted": req.muted});
            if let Some(result) = plugin.handle_command("set_app_mute", args) {
                if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return Json(ApiResponse::success("App mute set".to_string()));
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

    Json(ApiResponse::error("Volume plugin not available"))
}
