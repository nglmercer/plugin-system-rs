//! The timer plugin, compiled as a WASI Preview 2 component.
//!
//! This is the pilot for the FFI → WASI migration (see
//! `docs/wasi-migration.md`). It is deliberately the plugin with no native
//! dependencies, so it exercises the ABI, the host imports, and the loader
//! without needing any host capability beyond logging.
//!
//! Compared with the native `plugin-timer`, this version also implements
//! `handle-command`, since the whole point of the boundary is JSON commands.

wit_bindgen::generate!({
    // Point at the host's copy rather than vendoring a second one: a contract
    // with two definitions is a contract that drifts.
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

use std::cell::RefCell;
use std::collections::HashMap;

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::host::log as host_log;
use streamdeck::plugin::types::{CommandError, Dependency, LogLevel, Metadata};

// Each plugin gets its own `Store`, so a component instance is never shared
// between threads and thread-local state is exactly right here.
thread_local! {
    static TIMERS: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

struct TimerPlugin;

impl TimerPlugin {
    fn start(args: &serde_json::Value) -> Result<serde_json::Value, CommandError> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CommandError::InvalidArgs("expected a string `name`".into()))?;
        let seconds = args
            .get("seconds")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| CommandError::InvalidArgs("expected an integer `seconds`".into()))?;

        TIMERS.with(|t| t.borrow_mut().insert(name.to_string(), seconds));
        host_log(LogLevel::Info, &format!("started timer '{name}' at {seconds}s"));

        Ok(serde_json::json!({ "ok": true, "name": name, "seconds": seconds }))
    }

    fn get(args: &serde_json::Value) -> Result<serde_json::Value, CommandError> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CommandError::InvalidArgs("expected a string `name`".into()))?;

        let seconds = TIMERS.with(|t| t.borrow().get(name).copied());
        match seconds {
            Some(seconds) => Ok(serde_json::json!({ "ok": true, "name": name, "seconds": seconds })),
            None => Err(CommandError::NotFound(format!("no timer named '{name}'"))),
        }
    }

    fn stop(args: &serde_json::Value) -> Result<serde_json::Value, CommandError> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CommandError::InvalidArgs("expected a string `name`".into()))?;

        let existed = TIMERS.with(|t| t.borrow_mut().remove(name).is_some());
        Ok(serde_json::json!({ "ok": true, "removed": existed }))
    }

    fn list() -> serde_json::Value {
        let mut names = TIMERS.with(|t| t.borrow().keys().cloned().collect::<Vec<_>>());
        // Deterministic ordering keeps the widget from reshuffling on refresh.
        names.sort();
        serde_json::json!({ "ok": true, "timers": names })
    }

    fn snapshot() -> serde_json::Value {
        let mut timers: Vec<_> = TIMERS.with(|t| {
            t.borrow()
                .iter()
                .map(|(name, seconds)| serde_json::json!({ "name": name, "seconds": seconds }))
                .collect()
        });
        timers.sort_by_key(|v| v["name"].as_str().unwrap_or_default().to_string());
        serde_json::json!({ "timers": timers })
    }
}

impl Guest for TimerPlugin {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "timer".into(),
            version: "0.1.0".into(),
            authors: vec!["StreamDeck Core".into()],
            dependencies: Vec::<Dependency>::new(),
        }
    }

    fn on_load() {
        host_log(LogLevel::Info, "TimerPlugin loaded (wasm)");
    }

    fn on_unload() {
        host_log(LogLevel::Info, "TimerPlugin unloading (wasm)");
    }

    fn handle_command(method: String, args_json: String) -> Result<String, CommandError> {
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| CommandError::InvalidArgs(format!("args are not valid JSON: {e}")))?;

        let value = match method.as_str() {
            "start" => Self::start(&args)?,
            "get" => Self::get(&args)?,
            "stop" => Self::stop(&args)?,
            "list" => Self::list(),
            other => {
                return Err(CommandError::NotFound(format!(
                    "timer has no method '{other}'"
                )))
            }
        };

        serde_json::to_string(&value)
            .map_err(|e| CommandError::Failed(format!("failed to encode result: {e}")))
    }

    fn interface_ids() -> Vec<String> {
        vec!["Timer".into()]
    }

    fn interface_data() -> Option<String> {
        serde_json::to_string(&Self::snapshot()).ok()
    }
}

export!(TimerPlugin);
