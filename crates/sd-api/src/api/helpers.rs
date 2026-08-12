//! Calling into plugins from async request handlers.
//!
//! # Why everything here goes through `spawn_blocking`
//!
//! A plugin call is synchronous all the way down: `handle_command` enters
//! wasmtime, and any host capability it uses — a WebSocket handshake, an audio
//! query — blocks the calling thread until it returns. The OBS plugin's
//! `connect` performs a full handshake plus several request/response
//! round-trips with a 15 second timeout.
//!
//! Running that directly inside an `async fn` parks a tokio worker for the
//! duration. Worse, the handler holds the `PluginManager` read guard and the
//! plugin's own write guard the whole time, so one slow OBS connect serialised
//! every other request touching that plugin — including the ones that would
//! have told the user what was happening.
//!
//! So the helpers take the *shared* manager rather than a guard, and acquire
//! both locks inside a blocking thread. The async caller holds nothing while
//! it waits, and the runtime keeps serving.

use std::sync::Arc;

use plugin_system::PluginManager;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::response::ApiResponse;

/// The manager as handlers see it: shared, and locked only inside a blocking
/// thread.
pub type SharedPluginManager = Arc<RwLock<PluginManager>>;

/// Run `f` on the blocking pool with a read guard on the manager.
async fn with_manager<R, F>(pm: &SharedPluginManager, f: F) -> Option<R>
where
    F: FnOnce(&PluginManager) -> R + Send + 'static,
    R: Send + 'static,
{
    let pm = pm.clone();
    match tokio::task::spawn_blocking(move || {
        let manager = pm.blocking_read();
        f(&manager)
    })
    .await
    {
        Ok(value) => Some(value),
        Err(e) => {
            // A join error means the blocking task panicked. That is a bug in
            // the plugin boundary, not an expected "plugin unavailable", so it
            // is logged rather than folded silently into a None.
            log::error!("plugin call panicked on the blocking pool: {e}");
            None
        }
    }
}

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
/// This is the low-level helper: it takes both locks and enters the plugin on
/// a blocking thread, so the caller's tokio worker stays free.
pub async fn call_plugin_raw(
    plugin_manager: &SharedPluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> Option<Value> {
    let plugin_name = plugin_name.to_string();
    let method = method.to_string();
    with_manager(plugin_manager, move |manager| {
        call_plugin_raw_sync(manager, &plugin_name, &method, args)
    })
    .await
    .flatten()
}

/// Refresh a plugin, then read its interface data — in one blocking hop.
///
/// Both halves enter the plugin, so doing them as two separate awaits would
/// take and release the locks twice and let another caller interleave between
/// the refresh and the read, which is precisely the pair that must not be
/// split.
pub async fn refresh_and_read(
    plugin_manager: &SharedPluginManager,
    plugin_name: &str,
) -> Option<Value> {
    let plugin_name = plugin_name.to_string();
    with_manager(plugin_manager, move |manager| {
        let _ = call_plugin_ok_sync(manager, &plugin_name, "refresh", serde_json::json!({}));
        let plugin_arc = manager.get_plugin_arc(&plugin_name).ok()?;
        let plugin = plugin_arc.read().ok()?;
        plugin.interface_data()
    })
    .await
    .flatten()
}

/// Why a typed plugin call did not produce a value.
pub enum PluginCallError {
    /// The plugin is not loaded, or answered with nothing at all.
    Unavailable,
    /// The plugin answered `{"ok": false, "error": "..."}`. This is a normal,
    /// expected outcome (OBS closed, device unplugged, ...), not a bug.
    Reported(String),
    /// The plugin answered with a payload that does not match the contract.
    /// The mismatch itself is logged at error level by `call_plugin`.
    Shape,
}

impl PluginCallError {
    fn into_message(self, plugin_name: &str) -> String {
        match self {
            PluginCallError::Unavailable => format!("{plugin_name} plugin unavailable"),
            PluginCallError::Reported(msg) => msg,
            PluginCallError::Shape => {
                format!("{plugin_name} plugin returned an unexpected response")
            }
        }
    }
}

/// Pull the message out of an `{"ok": false, "error": "..."}` envelope.
///
/// Plugins use this shape to report expected failures, so it must be checked
/// *before* deserializing into the success type — otherwise every "OBS is not
/// running" answer looks like a contract violation.
fn reported_error(value: &Value) -> Option<String> {
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        Some(
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Unknown error")
                .to_string(),
        )
    } else {
        None
    }
}

/// Call a plugin command and deserialize the result into a typed response.
pub async fn call_plugin<T: DeserializeOwned>(
    plugin_manager: &SharedPluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> Result<T, PluginCallError> {
    let Some(result) = call_plugin_raw(plugin_manager, plugin_name, method, args).await else {
        return Err(PluginCallError::Unavailable);
    };

    if let Some(message) = reported_error(&result) {
        // Expected condition the caller renders; polling widgets hit this
        // every couple of seconds, so it must not be logged at error level.
        log::debug!("plugin '{plugin_name}' reported an error from '{method}': {message}");
        return Err(PluginCallError::Reported(message));
    }

    match serde_json::from_value(result.clone()) {
        Ok(value) => Ok(value),
        Err(e) => {
            // Distinguishing this from "no such plugin" matters: a shape
            // mismatch means the plugin answered and the contract drifted,
            // which is a different bug from the plugin being absent — and
            // silently returning None made it look like the latter.
            log::error!(
                "plugin '{plugin_name}' returned an unexpected shape from '{method}': {e}; got {result}"
            );
            Err(PluginCallError::Shape)
        }
    }
}

/// Call a plugin command that returns `{"ok": true}` or `{"ok": false, "error": "..."}`.
///
/// Returns `Ok(())` on success, or `Err(error_message)` on failure.
pub async fn call_plugin_ok(
    plugin_manager: &SharedPluginManager,
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
    plugin_manager: &SharedPluginManager,
    plugin_name: &str,
    method: &str,
    args: Value,
) -> ApiResponse<T> {
    match call_plugin::<T>(plugin_manager, plugin_name, method, args).await {
        Ok(data) => ApiResponse::success(data),
        // A plugin-reported failure keeps its own message ("not connected to
        // OBS"), so the UI can show the actual reason instead of a generic
        // "plugin unavailable" that points at the wrong thing.
        Err(e) => ApiResponse::error(e.into_message(plugin_name)),
    }
}

/// Call a plugin command that returns `{"ok": true}` and return an ApiResponse.
pub async fn call_plugin_ok_response(
    plugin_manager: &SharedPluginManager,
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
        $crate::api::helpers::call_plugin_raw(&pm, $plugin, $method, $args).await
    }};
}

/// Macro for calling plugin commands and getting typed responses.
///
/// Usage:
/// ```ignore
/// let data: Result<ObsStatusResponse, _> = plugin_call_typed!(state, "obs", "get_status", {});
/// ```
#[macro_export]
macro_rules! plugin_call_typed {
    ($state:expr, $plugin:expr, $method:expr, $args:expr, $ty:ty) => {{
        let pm = $state.plugin_manager.plugin_manager();
        $crate::api::helpers::call_plugin::<$ty>(&pm, $plugin, $method, $args).await
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
        $crate::api::helpers::call_plugin_ok_response(&pm, $plugin, $method, $args, $success)
            .await
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failure envelope plugins use for expected conditions must be read
    /// as a reported error, not fed to the success deserializer — that is what
    /// turned "not connected to OBS" into an error-level shape complaint on
    /// every poll.
    #[test]
    fn recognizes_the_plugin_failure_envelope() {
        let payload = serde_json::json!({"ok": false, "error": "not connected to OBS"});
        assert_eq!(
            reported_error(&payload).as_deref(),
            Some("not connected to OBS")
        );
    }

    #[test]
    fn a_failure_envelope_without_a_message_still_reports() {
        let payload = serde_json::json!({"ok": false});
        assert_eq!(reported_error(&payload).as_deref(), Some("Unknown error"));
    }

    /// Success payloads must pass through untouched, including ones that
    /// happen to carry an `ok: true` marker or no `ok` key at all.
    #[test]
    fn success_payloads_are_not_mistaken_for_failures() {
        assert!(reported_error(&serde_json::json!({"ok": true})).is_none());
        assert!(reported_error(&serde_json::json!({"current_scene": "Escena"})).is_none());
        assert!(reported_error(&serde_json::json!([])).is_none());
        assert!(reported_error(&serde_json::json!(false)).is_none());
    }

    /// The UI shows this string, so a reported failure keeps the plugin's own
    /// wording instead of a generic "unavailable".
    #[test]
    fn reported_errors_keep_their_message() {
        assert_eq!(
            PluginCallError::Reported("not connected to OBS".into()).into_message("obs"),
            "not connected to OBS"
        );
        assert_eq!(
            PluginCallError::Unavailable.into_message("obs"),
            "obs plugin unavailable"
        );
    }
}
