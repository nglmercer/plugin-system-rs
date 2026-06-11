use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::{api::system::system_stats_data, response::ApiResponse, state::AppState};

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

    match manager.get_plugin_info(&plugin_name) {
        Ok(info) => {
            let data = match plugin_name.as_str() {
                "system-monitor" => system_stats_data(),
                _ => serde_json::json!({}),
            };

            Json(ApiResponse::success(PluginDataResponse {
                name: info.name,
                version: info.version,
                interfaces: info.interfaces,
                data,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
