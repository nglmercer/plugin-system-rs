use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};

#[derive(Serialize)]
pub(crate) struct VolumeStateResponse {
    master_volume: f32,
    muted: bool,
    default_device_name: String,
    platform_supported: bool,
    per_app_supported: bool,
}

#[derive(Serialize)]
pub(crate) struct AppVolumeResponse {
    name: String,
    volume: f32,
    muted: bool,
    pid: Option<u32>,
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

pub(crate) async fn get_volume_state(
    State(state): State<AppState>,
) -> Json<ApiResponse<VolumeStateResponse>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.with_plugin("volume-master", |plugin| {
        if let Some(vol_plugin) = plugin.downcast_ref::<plugin_volume_master::VolumeMasterPlugin>()
        {
            let data = &vol_plugin.data;
            Ok(VolumeStateResponse {
                master_volume: data.state.master_volume,
                muted: data.state.muted,
                default_device_name: data.state.default_device_name.clone(),
                platform_supported: data.platform_supported,
                per_app_supported: data.per_app_supported,
            })
        } else {
            Err("Volume plugin not available".to_string())
        }
    }) {
        Ok(Ok(resp)) => Json(ApiResponse::success(resp)),
        Ok(Err(e)) => Json(ApiResponse::error(e)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn get_app_volumes(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<AppVolumeResponse>>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.with_plugin("volume-master", |plugin| {
        if let Some(vol_plugin) = plugin.downcast_ref::<plugin_volume_master::VolumeMasterPlugin>()
        {
            Ok(vol_plugin
                .data
                .apps
                .iter()
                .map(|a| AppVolumeResponse {
                    name: a.name.clone(),
                    volume: a.volume,
                    muted: a.muted,
                    pid: a.pid,
                })
                .collect())
        } else {
            Err("Volume plugin not available".to_string())
        }
    }) {
        Ok(Ok(apps)) => Json(ApiResponse::success(apps)),
        Ok(Err(e)) => Json(ApiResponse::error(e)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn set_master_volume(
    State(state): State<AppState>,
    Json(req): Json<SetVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.with_plugin_mut("volume-master", |plugin| {
        if let Some(vol_plugin) = plugin.downcast_mut::<plugin_volume_master::VolumeMasterPlugin>()
        {
            vol_plugin
                .set_volume(req.volume)
                .map(|_| "Volume set".to_string())
        } else {
            Err("Volume plugin not available".to_string())
        }
    }) {
        Ok(Ok(msg)) => Json(ApiResponse::success(msg)),
        Ok(Err(e)) => Json(ApiResponse::error(e)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn set_master_mute(
    State(state): State<AppState>,
    Json(req): Json<SetMuteRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.with_plugin_mut("volume-master", |plugin| {
        if let Some(vol_plugin) = plugin.downcast_mut::<plugin_volume_master::VolumeMasterPlugin>()
        {
            vol_plugin
                .set_muted(req.muted)
                .map(|_| "Mute set".to_string())
        } else {
            Err("Volume plugin not available".to_string())
        }
    }) {
        Ok(Ok(msg)) => Json(ApiResponse::success(msg)),
        Ok(Err(e)) => Json(ApiResponse::error(e)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn set_app_volume(
    State(state): State<AppState>,
    Json(req): Json<SetAppVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.with_plugin_mut("volume-master", |plugin| {
        if let Some(vol_plugin) = plugin.downcast_mut::<plugin_volume_master::VolumeMasterPlugin>()
        {
            vol_plugin
                .set_app_volume(&req.app_name, req.volume)
                .map(|_| "App volume set".to_string())
        } else {
            Err("Volume plugin not available".to_string())
        }
    }) {
        Ok(Ok(msg)) => Json(ApiResponse::success(msg)),
        Ok(Err(e)) => Json(ApiResponse::error(e)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn set_app_mute(
    State(state): State<AppState>,
    Json(req): Json<SetAppMuteRequest>,
) -> Json<ApiResponse<String>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.with_plugin_mut("volume-master", |plugin| {
        if let Some(vol_plugin) = plugin.downcast_mut::<plugin_volume_master::VolumeMasterPlugin>()
        {
            vol_plugin
                .set_app_muted(&req.app_name, req.muted)
                .map(|_| "App mute set".to_string())
        } else {
            Err("Volume plugin not available".to_string())
        }
    }) {
        Ok(Ok(msg)) => Json(ApiResponse::success(msg)),
        Ok(Err(e)) => Json(ApiResponse::error(e)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
