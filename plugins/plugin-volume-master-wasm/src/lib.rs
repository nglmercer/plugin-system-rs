//! Master and per-application volume control, as a WASI Preview 2 component.
//!
//! The PulseAudio / COM / CoreAudio backends that used to be compiled into
//! this plugin now live in the host behind the `audio` capability. What
//! remains here is what was always this plugin's own work: the command
//! surface, the cached snapshot widgets poll, and the optimistic local updates
//! that keep a slider from snapping back while the device catches up.
//!
//! Honest note on the split: this plugin is thin. Most of what it used to
//! *be* is now host code, and the boundary buys sandboxing around logic that
//! is mostly forwarding. It stays a plugin because the command surface and the
//! widget contract are genuinely its own, and because the host capability is
//! now reusable by anything else that wants audio.

wit_bindgen::generate!({
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

use std::cell::RefCell;

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::audio;
use streamdeck::plugin::host::log as host_log;
use streamdeck::plugin::types::{CommandError, Dependency, LogLevel, Metadata};

use serde::Serialize;

/// Master volume, in the shape widgets already expect.
#[derive(Debug, Clone, Default, Serialize)]
struct VolumeState {
    master_volume: f32,
    muted: bool,
    default_device_name: String,
}

#[derive(Debug, Clone, Serialize)]
struct AppVolume {
    /// Addresses this exact stream. One app can own several — a browser plays
    /// one per tab — so commands carry this rather than the app name.
    id: String,
    name: String,
    /// What the stream is playing, e.g. a tab title. Empty when unknown.
    title: String,
    /// Freedesktop icon name; the frontend resolves it via `/api/icon/:name`.
    icon: String,
    volume: f32,
    muted: bool,
    pid: Option<u32>,
}

/// The full payload handed to `interface-data`. Field names are unchanged from
/// the native plugin so the frontend did not need touching.
#[derive(Debug, Clone, Default, Serialize)]
struct VolumeData {
    state: VolumeState,
    apps: Vec<AppVolume>,
    platform_supported: bool,
    per_app_supported: bool,
}

thread_local! {
    static DATA: RefCell<VolumeData> = RefCell::new(VolumeData::default());
}

struct VolumeMasterPlugin;

impl VolumeMasterPlugin {
    /// Re-read everything from the host.
    ///
    /// Individual failures are swallowed rather than propagated: a machine
    /// with master volume but no per-app support should still report the half
    /// that works, which is also why `support` is consulted separately.
    fn refresh() {
        let support = audio::get_support();

        DATA.with(|d| {
            let mut data = d.borrow_mut();
            data.platform_supported = support.master;
            data.per_app_supported = support.per_app;

            if support.master {
                match audio::get_master() {
                    Ok(s) => {
                        data.state = VolumeState {
                            master_volume: s.volume,
                            muted: s.muted,
                            default_device_name: s.device_name,
                        }
                    }
                    Err(e) => host_log(LogLevel::Warn, &format!("get-master failed: {e}")),
                }
            }

            if support.per_app {
                match audio::list_apps() {
                    Ok(apps) => {
                        data.apps = apps
                            .into_iter()
                            .map(|a| AppVolume {
                                id: a.id,
                                name: a.name,
                                title: a.title,
                                icon: a.icon,
                                volume: a.volume,
                                muted: a.muted,
                                pid: a.pid,
                            })
                            .collect()
                    }
                    Err(e) => host_log(LogLevel::Warn, &format!("list-apps failed: {e}")),
                }
            }
        });
    }

    fn arg_f32(args: &serde_json::Value, key: &str) -> Result<f32, CommandError> {
        args.get(key)
            .and_then(|v| v.as_f64())
            .map(|v| (v as f32).clamp(0.0, 100.0))
            .ok_or_else(|| CommandError::InvalidArgs(format!("expected a number `{key}`")))
    }

    fn arg_bool(args: &serde_json::Value, key: &str) -> Result<bool, CommandError> {
        args.get(key)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| CommandError::InvalidArgs(format!("expected a boolean `{key}`")))
    }

    /// Which stream a command targets.
    ///
    /// Prefers `app_id`, falling back to `app_name` so a caller written
    /// against the old command shape keeps working — at the cost of ambiguity
    /// when one app owns several streams, which is exactly what `app_id`
    /// exists to resolve.
    fn arg_target(args: &serde_json::Value) -> Result<String, CommandError> {
        if let Some(id) = args.get("app_id").and_then(|v| v.as_str()) {
            if !id.is_empty() {
                return Ok(id.to_string());
            }
        }
        Self::arg_str(args, "app_name")
    }

    /// Match the cached entry a command just changed, by id then by name.
    fn find_app<'a>(apps: &'a mut [AppVolume], target: &str) -> Option<&'a mut AppVolume> {
        if let Some(index) = apps.iter().position(|a| a.id == target) {
            return apps.get_mut(index);
        }
        apps.iter_mut().find(|a| a.name == target)
    }

    fn arg_str(args: &serde_json::Value, key: &str) -> Result<String, CommandError> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| CommandError::InvalidArgs(format!("expected a string `{key}`")))
    }
}

impl Guest for VolumeMasterPlugin {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "volume-master".into(),
            version: "0.1.0".into(),
            authors: vec!["StreamDeck Core".into()],
            dependencies: Vec::<Dependency>::new(),
        }
    }

    fn on_load() {
        host_log(LogLevel::Info, "VolumeMasterPlugin loaded (wasm)");
        Self::refresh();
    }

    fn on_unload() {
        host_log(LogLevel::Info, "VolumeMasterPlugin unloading (wasm)");
    }

    fn interface_ids() -> Vec<String> {
        vec!["VolumeMaster".into()]
    }

    fn interface_data() -> Option<String> {
        DATA.with(|d| serde_json::to_string(&*d.borrow()).ok())
    }

    fn handle_command(method: String, args_json: String) -> Result<String, CommandError> {
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| CommandError::InvalidArgs(format!("args are not valid JSON: {e}")))?;

        let value = match method.as_str() {
            "refresh" => {
                Self::refresh();
                serde_json::json!({ "ok": true })
            }

            "set_volume" => {
                let volume = Self::arg_f32(&args, "volume")?;
                audio::set_master(volume).map_err(CommandError::Failed)?;
                // Applied locally too, so a slider dragged quickly does not
                // jump back to a stale value on the next poll.
                DATA.with(|d| d.borrow_mut().state.master_volume = volume);
                serde_json::json!({ "ok": true })
            }

            "set_mute" => {
                let muted = Self::arg_bool(&args, "muted")?;
                audio::set_master_mute(muted).map_err(CommandError::Failed)?;
                DATA.with(|d| d.borrow_mut().state.muted = muted);
                serde_json::json!({ "ok": true })
            }

            "set_app_volume" => {
                let target = Self::arg_target(&args)?;
                let volume = Self::arg_f32(&args, "volume")?;
                audio::set_app_volume(&target, volume).map_err(CommandError::Failed)?;
                DATA.with(|d| {
                    if let Some(app) = Self::find_app(&mut d.borrow_mut().apps, &target) {
                        app.volume = volume;
                    }
                });
                serde_json::json!({ "ok": true })
            }

            "set_app_mute" => {
                let target = Self::arg_target(&args)?;
                let muted = Self::arg_bool(&args, "muted")?;
                audio::set_app_mute(&target, muted).map_err(CommandError::Failed)?;
                DATA.with(|d| {
                    if let Some(app) = Self::find_app(&mut d.borrow_mut().apps, &target) {
                        app.muted = muted;
                    }
                });
                serde_json::json!({ "ok": true })
            }

            other => {
                return Err(CommandError::NotFound(format!(
                    "volume-master has no method '{other}'"
                )))
            }
        };

        serde_json::to_string(&value)
            .map_err(|e| CommandError::Failed(format!("failed to serialize response: {e}")))
    }
}

export!(VolumeMasterPlugin);
