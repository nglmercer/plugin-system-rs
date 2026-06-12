use plugin_system::PluginManager;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::response::ApiResponse;

/// Call a plugin command synchronously (for use in spawn_blocking).
pub fn call_plugin_raw_sync(
    plugin_manager: &PluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> Option<Value> {
    plugin_manager
        .with_plugin_mut(plugin_name, |plugin| plugin.handle_command(method, args))
        .ok()
        .flatten()
}

/// Call a plugin command synchronously that returns ok/error.
pub fn call_plugin_ok_sync(
    plugin_manager: &PluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> Result<(), String> {
    let result = call_plugin_raw_sync(plugin_manager, plugin_name, method, args);
    match result {
        Some(data) => {
            if data.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                Ok(())
            } else {
                Err(data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string())
            }
        }
        None => Err(format!("{} plugin not available", plugin_name)),
    }
}

/// Call a plugin command and return the raw JSON result.
///
/// This is the low-level helper that handles the plugin manager locking pattern.
pub async fn call_plugin_raw(
    plugin_manager: &PluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> Option<Value> {
    plugin_manager
        .with_plugin_mut(plugin_name, |plugin| plugin.handle_command(method, args))
        .ok()
        .flatten()
}

/// Call a plugin command and deserialize the result into a typed response.
///
/// Returns `None` if the plugin is not available or the result cannot be deserialized.
pub async fn call_plugin<T: DeserializeOwned>(
    plugin_manager: &PluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> Option<T> {
    let result = call_plugin_raw(plugin_manager, plugin_name, method, args).await?;
    serde_json::from_value(result).ok()
}

/// Call a plugin command that returns `{"ok": true}` or `{"ok": false, "error": "..."}`.
///
/// Returns `Ok(())` on success, or `Err(error_message)` on failure.
pub async fn call_plugin_ok(
    plugin_manager: &PluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> Result<(), String> {
    let result = call_plugin_raw(plugin_manager, plugin_name, method, args).await;
    match result {
        Some(data) => {
            if data.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                Ok(())
            } else {
                Err(data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string())
            }
        }
        None => Err(format!("{} plugin not available", plugin_name)),
    }
}

/// Call a plugin command and return an ApiResponse.
///
/// On success, wraps the result in `ApiResponse::success`.
/// On failure, returns `ApiResponse::error` with the plugin name.
pub async fn call_plugin_response<T: DeserializeOwned + serde::Serialize>(
    plugin_manager: &PluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> ApiResponse<T> {
    match call_plugin::<T>(plugin_manager, plugin_name, method, args).await {
        Some(data) => ApiResponse::success(data),
        None => ApiResponse::error(format!("{} plugin not available", plugin_name)),
    }
}

/// Call a plugin command that returns `{"ok": true}` and return an ApiResponse.
pub async fn call_plugin_ok_response(
    plugin_manager: &PluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
    success_msg: &str,
) -> ApiResponse<String> {
    match call_plugin_ok(plugin_manager, plugin_name, method, args).await {
        Ok(()) => ApiResponse::success(success_msg.to_string()),
        Err(e) => ApiResponse::error(e),
    }
}

/// Macro for calling plugin commands in API handlers.
///
/// Usage:
/// ```ignore
/// let result = plugin_call!(state, "obs", "get_status", {});
/// ```
#[macro_export]
macro_rules! plugin_call {
    ($state:expr, $plugin:expr, $method:expr, $args:expr) => {{
        let pm = $state.plugin_manager.plugin_manager();
        let manager = pm.read().await;
        $crate::api::helpers::call_plugin_raw(&manager, $plugin, $method, $args).await
    }};
}

/// Macro for calling plugin commands and getting typed responses.
///
/// Usage:
/// ```ignore
/// let data: Option<ObsStatusResponse> = plugin_call_typed!(state, "obs", "get_status", {});
/// ```
#[macro_export]
macro_rules! plugin_call_typed {
    ($state:expr, $plugin:expr, $method:expr, $args:expr, $ty:ty) => {{
        let pm = $state.plugin_manager.plugin_manager();
        let manager = pm.read().await;
        $crate::api::helpers::call_plugin::<$ty>(&manager, $plugin, $method, $args).await
    }};
}

/// Macro for calling plugin commands that return ok/error and returning an ApiResponse.
///
/// Usage:
/// ```ignore
/// return plugin_call_ok_response!(state, "obs", "connect", args, "Connected to OBS");
/// ```
#[macro_export]
macro_rules! plugin_call_ok_response {
    ($state:expr, $plugin:expr, $method:expr, $args:expr, $success:expr) => {{
        let pm = $state.plugin_manager.plugin_manager();
        let manager = pm.read().await;
        $crate::api::helpers::call_plugin_ok_response(&manager, $plugin, $method, $args, $success).await
    }};
}
