use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};
use sd_plugins::PluginStatus;

#[derive(Serialize)]
pub(crate) struct PluginDataResponse {
    name: String,
    version: String,
    interfaces: Vec<String>,
    data: serde_json::Value,
}

/// The parts of a plugin upload, before any policy has been applied.
struct UploadedPluginFile {
    filename: String,
    bytes: Vec<u8>,
    /// The plugin's sidecar manifest, as JSON. Optional: a plugin that needs
    /// no host capabilities needs no manifest, and the absence of one is the
    /// most constrained case rather than an error.
    manifest_json: Option<String>,
    /// Capability names the caller explicitly agreed to grant, from repeated
    /// `acknowledge_capability` fields or one comma-separated
    /// `acknowledge_capabilities` field.
    acknowledged_capabilities: Vec<String>,
    enabled: Option<bool>,
}

async fn read_plugin_upload(mut multipart: Multipart) -> Result<UploadedPluginFile, String> {
    let mut file: Option<(String, Vec<u8>)> = None;
    let mut manifest_json = None;
    let mut acknowledged_capabilities = Vec::new();
    let mut enabled = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| format!("Failed to read multipart field: {e}"))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                // A component is always `.wasm`; the old `plugin.dll` default
                // could only ever produce a confusing rejection downstream.
                let filename = field.file_name().unwrap_or("plugin.wasm").to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| format!("Failed to read plugin file: {e}"))?;
                file = Some((filename, bytes.to_vec()));
            }
            "manifest" => {
                manifest_json = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| format!("Failed to read manifest field: {e}"))?,
                );
            }
            "acknowledge_capability" => {
                let value = field
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read acknowledge_capability field: {e}"))?;
                acknowledged_capabilities.push(value.trim().to_string());
            }
            "acknowledge_capabilities" => {
                let value = field
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read acknowledge_capabilities field: {e}"))?;
                acknowledged_capabilities.extend(
                    value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty()),
                );
            }
            "enabled" => {
                let value = field
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read enabled field: {e}"))?;
                enabled = Some(value != "false" && value != "0");
            }
            _ => {}
        }
    }

    let (filename, bytes) = file.ok_or_else(|| "Missing multipart file field".to_string())?;
    Ok(UploadedPluginFile {
        filename,
        bytes,
        manifest_json,
        acknowledged_capabilities,
        enabled,
    })
}

impl UploadedPluginFile {
    fn as_request(&self) -> sd_plugins::PluginUpload<'_> {
        sd_plugins::PluginUpload {
            bytes: &self.bytes,
            filename: &self.filename,
            manifest_json: self.manifest_json.as_deref(),
            acknowledged_capabilities: &self.acknowledged_capabilities,
            enabled: self.enabled,
        }
    }
}

fn plugin_api_error(error: impl ToString) -> String {
    error.to_string()
}

pub(crate) async fn list_plugins(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<PluginStatus>>> {
    match state.plugin_manager.list_plugin_statuses().await {
        Ok(statuses) => Json(ApiResponse::success(statuses)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn uninstall_plugin(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Json<ApiResponse<String>> {
    match state.plugin_manager.uninstall_plugin(&plugin_name).await {
        Ok(()) => Json(ApiResponse::success("Plugin uninstalled".to_string())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn reload_plugins(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    match state.plugin_manager.reload_plugins().await {
        Ok(()) => Json(ApiResponse::success("Plugins reloaded".to_string())),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

pub(crate) async fn refresh_plugins(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    match state.plugin_manager.refresh_plugins_from_dir().await {
        Ok(_) => Json(ApiResponse::success("Plugins refreshed".to_string())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn upload_plugin(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Json<ApiResponse<PluginStatus>> {
    let file = match read_plugin_upload(multipart).await {
        Ok(value) => value,
        Err(e) => return Json(ApiResponse::error(e)),
    };

    match state
        .plugin_manager
        .install_plugin_file(file.as_request())
        .await
    {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => Json(ApiResponse::error(plugin_api_error(e))),
    }
}

pub(crate) async fn update_plugin(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
    multipart: Multipart,
) -> Json<ApiResponse<PluginStatus>> {
    let file = match read_plugin_upload(multipart).await {
        Ok(value) => value,
        Err(e) => return Json(ApiResponse::error(e)),
    };

    match state
        .plugin_manager
        .update_plugin_file(&plugin_name, file.as_request())
        .await
    {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => Json(ApiResponse::error(plugin_api_error(e))),
    }
}

pub(crate) async fn get_plugin_data(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Json<ApiResponse<PluginDataResponse>> {
    let pm = state.plugin_manager.plugin_manager();

    // `interface_data` enters the guest, so this belongs on the blocking pool
    // like every other plugin call.
    let name = plugin_name.clone();
    let result = tokio::task::spawn_blocking(move || {
        let manager = pm.blocking_read();
        let plugin_arc = manager.get_plugin_arc(&name).map_err(|e| e.to_string())?;
        let plugin = plugin_arc.read().map_err(|_| "plugin lock poisoned".to_string())?;
        let meta = plugin.metadata();
        Ok::<_, String>(PluginDataResponse {
            name: meta.name,
            version: meta.version,
            interfaces: plugin.interface_ids(),
            data: plugin
                .interface_data()
                .unwrap_or_else(|| serde_json::json!({})),
        })
    })
    .await;

    match result {
        Ok(Ok(data)) => Json(ApiResponse::success(data)),
        Ok(Err(e)) => Json(ApiResponse::error(e)),
        Err(e) => {
            log::error!("reading plugin data for '{plugin_name}' panicked: {e}");
            Json(ApiResponse::error(format!(
                "Reading plugin '{plugin_name}' failed"
            )))
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct PluginCommandRequest {
    method: String,
    #[serde(default)]
    args: serde_json::Value,
}

/// Invoke a command on a loaded plugin.
///
/// The typed endpoints (`/api/obs/*`, `/api/volume/*`) exist because their
/// payloads have a contract worth enforcing. A plugin outside that set — the
/// timer, or anything a user installs — otherwise has no route at all and is
/// invisible to the dashboard, which is how the timer plugin ended up shipped
/// with no way to reach it. This is the general path for those.
pub(crate) async fn call_plugin_command(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
    Json(req): Json<PluginCommandRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let pm = state.plugin_manager.plugin_manager();

    let args = if req.args.is_null() {
        serde_json::json!({})
    } else {
        req.args
    };

    match crate::api::helpers::call_plugin_raw(&pm, &plugin_name, &req.method, args).await {
        Some(value) => Json(ApiResponse::success(value)),
        None => Json(ApiResponse::error(format!(
            "Plugin '{plugin_name}' is not loaded, or rejected '{}'",
            req.method
        ))),
    }
}

#[derive(Deserialize)]
pub(crate) struct SetEnabledRequest {
    enabled: bool,
}

pub(crate) async fn set_plugin_enabled(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
    Json(req): Json<SetEnabledRequest>,
) -> Json<ApiResponse<PluginStatus>> {
    match state
        .plugin_manager
        .set_plugin_enabled(plugin_name.clone(), req.enabled)
        .await
    {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
