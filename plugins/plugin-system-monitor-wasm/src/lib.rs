//! System resource monitoring, as a WASI Preview 2 component.
//!
//! The `sysinfo` dependency that used to sit inside this plugin now lives in
//! the host, behind the `system-info` capability: a component cannot read
//! `/proc`, and pretending otherwise was never an option.
//!
//! What stayed guest-side is everything that was actually *this plugin's* job
//! — caching the last sample so widgets can poll cheaply, deriving the
//! percentage the UI wants, and shaping the JSON. The host answers "what is
//! the machine doing"; the plugin decides what to do with the answer.

wit_bindgen::generate!({
    // The host's copy is the single source of truth; vendoring a second one
    // would be a contract that drifts.
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

use std::cell::RefCell;

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::host::log as host_log;
use streamdeck::plugin::system_info::{get_stats, SystemStats};
use streamdeck::plugin::types::{CommandError, Dependency, LogLevel, Metadata};

use serde::Serialize;

/// What widgets receive. Mirrors the old native plugin's payload exactly, so
/// the frontend did not have to change.
#[derive(Debug, Clone, Default, Serialize)]
struct Stats {
    cpu_usage: f64,
    cpu_model: String,
    cpu_cores: u32,
    memory_total: u64,
    memory_used: u64,
    /// Derived here rather than in the host: it is a presentation concern.
    memory_usage: f64,
    swap_total: u64,
    swap_used: u64,
    load_avg: [f64; 3],
    uptime: u64,
    process_count: u32,
    thread_count: u32,
}

impl From<SystemStats> for Stats {
    fn from(s: SystemStats) -> Self {
        let memory_usage = if s.memory_total > 0 {
            s.memory_used as f64 / s.memory_total as f64 * 100.0
        } else {
            0.0
        };

        Self {
            cpu_usage: s.cpu_usage,
            cpu_model: s.cpu_model,
            cpu_cores: s.cpu_cores,
            memory_total: s.memory_total,
            memory_used: s.memory_used,
            memory_usage,
            swap_total: s.swap_total,
            swap_used: s.swap_used,
            load_avg: [s.load_avg_one, s.load_avg_five, s.load_avg_fifteen],
            uptime: s.uptime_seconds,
            process_count: s.process_count,
            thread_count: s.thread_count,
        }
    }
}

// One `Store` per plugin means the instance is never shared across threads,
// so thread-local state is exactly right.
thread_local! {
    static STATS: RefCell<Stats> = RefCell::new(Stats::default());
}

struct SystemMonitorPlugin;

impl SystemMonitorPlugin {
    /// Sample the host and cache the result.
    ///
    /// Kept explicit rather than sampling on every read: `get-stats` blocks
    /// briefly to compute a CPU delta, and a widget polling `interface-data`
    /// twice a second should not pay that each time.
    fn refresh() -> Result<(), String> {
        let stats: Stats = get_stats()?.into();
        STATS.with(|s| *s.borrow_mut() = stats);
        Ok(())
    }
}

impl Guest for SystemMonitorPlugin {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "system-monitor".into(),
            version: "0.1.0".into(),
            authors: vec!["StreamDeck Core".into()],
            dependencies: Vec::<Dependency>::new(),
        }
    }

    fn on_load() {
        host_log(LogLevel::Info, "SystemMonitorPlugin loaded (wasm)");
        // A failure here is not fatal: the plugin comes up reporting zeroes
        // and the next `refresh` can succeed.
        if let Err(e) = Self::refresh() {
            host_log(
                LogLevel::Warn,
                &format!("initial system-info sample failed: {e}"),
            );
        }
    }

    fn on_unload() {
        host_log(LogLevel::Info, "SystemMonitorPlugin unloading (wasm)");
    }

    fn interface_ids() -> Vec<String> {
        vec!["SystemMonitor".into()]
    }

    fn interface_data() -> Option<String> {
        STATS.with(|s| serde_json::to_string(&*s.borrow()).ok())
    }

    fn handle_command(method: String, args_json: String) -> Result<String, CommandError> {
        // Parsed even when unused, so a malformed payload is rejected
        // consistently rather than only on the commands that read it.
        let _args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| CommandError::InvalidArgs(format!("args are not valid JSON: {e}")))?;

        let value = match method.as_str() {
            "refresh" => {
                Self::refresh().map_err(CommandError::Failed)?;
                STATS.with(|s| serde_json::json!({ "ok": true, "stats": &*s.borrow() }))
            }
            "get" => STATS.with(|s| serde_json::json!({ "ok": true, "stats": &*s.borrow() })),
            other => {
                return Err(CommandError::NotFound(format!(
                    "system-monitor has no method '{other}'"
                )))
            }
        };

        serde_json::to_string(&value)
            .map_err(|e| CommandError::Failed(format!("failed to serialize response: {e}")))
    }
}

export!(SystemMonitorPlugin);
