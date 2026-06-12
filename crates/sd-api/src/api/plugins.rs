use axum::{
    extract::{Path, State},
    Json,
};
use plugin_key_simulator::KeySimulatorPlugin;
use plugin_obs::ObsPlugin;
use plugin_system_monitor::SystemMonitorPlugin;
use plugin_volume_master::VolumeMasterPlugin;
use serde::Serialize;

use crate::{response::ApiResponse, state::AppState};

#[derive(Serialize)]
pub(crate) struct PluginDataResponse {
    name: String,
    version: String,
    interfaces: Vec<String>,
    data: serde_json::Value,
}

pub(crate) async fn list_plugins(State(state): State<AppState>) -> Json<ApiResponse<Vec<String>>> {
    let plugins = state.plugin_manager.list_plugins().await;
    Json(ApiResponse::success(plugins))
}

pub(crate) async fn reload_plugins(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    match state.plugin_manager.reload_plugins().await {
        Ok(()) => Json(ApiResponse::success("Plugins reloaded".to_string())),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

pub(crate) async fn get_plugin_data(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Json<ApiResponse<PluginDataResponse>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.get_plugin_arc(&plugin_name) {
        Ok(plugin_arc) => {
            let plugin = plugin_arc.read().expect("plugin lock poisoned");
            let meta = plugin.metadata();

            let interfaces: Vec<String> = match plugin_name.as_str() {
                "obs" => plugin
                    .downcast_ref::<ObsPlugin>()
                    .map(|p| p.interface_ids().into_iter().map(String::from).collect())
                    .unwrap_or_default(),
                "volume-master" => plugin
                    .downcast_ref::<VolumeMasterPlugin>()
                    .map(|p| p.interface_ids().into_iter().map(String::from).collect())
                    .unwrap_or_default(),
                "system-monitor" => plugin
                    .downcast_ref::<SystemMonitorPlugin>()
                    .map(|p| p.interface_ids().into_iter().map(String::from).collect())
                    .unwrap_or_default(),
                "key-simulator" => plugin
                    .downcast_ref::<KeySimulatorPlugin>()
                    .map(|p| p.interface_ids().into_iter().map(String::from).collect())
                    .unwrap_or_default(),
                _ => Vec::new(),
            };

            let data = match plugin_name.as_str() {
                "obs" => plugin
                    .downcast_ref::<ObsPlugin>()
                    .and_then(|p| p.interface_data()),
                "volume-master" => plugin
                    .downcast_ref::<VolumeMasterPlugin>()
                    .and_then(|p| p.interface_data()),
                "system-monitor" => plugin
                    .downcast_ref::<SystemMonitorPlugin>()
                    .and_then(|p| p.interface_data()),
                _ => None,
            }
            .unwrap_or_else(|| serde_json::json!({}));

            Json(ApiResponse::success(PluginDataResponse {
                name: meta.name,
                version: meta.version,
                interfaces,
                data,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
