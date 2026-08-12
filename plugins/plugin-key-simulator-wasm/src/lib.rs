//! Keyboard simulation and hotkey recording, as a WASI Preview 2 component.
//!
//! The `rdev` dependency moved into the host behind the `input` capability: a
//! component reaches neither X11, uinput, nor the Win32 input queue.
//!
//! This plugin is a thin one, and worth being honest about. What it keeps is
//! the command surface and the argument handling — splitting a caller's flat
//! key list into modifiers and a main key, which is a policy decision the host
//! interface deliberately does not make for it.
//!
//! # Capability note
//!
//! `input` is the sharpest grant in the system: it can both type into whatever
//! window has focus and watch everything typed. A deployment that does not
//! need hotkeys should not grant it.

wit_bindgen::generate!({
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::host::log as host_log;
use streamdeck::plugin::input;
use streamdeck::plugin::types::{CommandError, Dependency, LogLevel, Metadata};

/// Names the host treats as modifiers, in the casing callers use.
const MODIFIERS: &[&str] = &["ctrl", "shift", "alt", "altgr", "win", "meta", "super"];

fn is_modifier(key: &str) -> bool {
    MODIFIERS.contains(&key.to_lowercase().as_str())
}

struct KeySimulatorPlugin;

impl KeySimulatorPlugin {
    /// Split a flat key list into held modifiers and the key to tap.
    ///
    /// Callers send `["ctrl", "shift", "a"]` without saying which is which,
    /// so the split happens here rather than in the capability: the host
    /// interface takes an explicit `modifiers` list precisely so it does not
    /// have to guess.
    fn split_chord(keys: &[String]) -> Result<(Vec<String>, String), CommandError> {
        let (mods, mains): (Vec<_>, Vec<_>) =
            keys.iter().cloned().partition(|k| is_modifier(k));

        match mains.len() {
            // All modifiers and nothing to press: treat the last as the key,
            // so "ctrl+shift" still does something sensible.
            0 => {
                let mut mods = mods;
                let main = mods.pop().ok_or_else(|| {
                    CommandError::InvalidArgs("expected at least one key".into())
                })?;
                Ok((mods, main))
            }
            _ => Ok((mods, mains[0].clone())),
        }
    }

    fn arg_keys(args: &serde_json::Value) -> Result<Vec<String>, CommandError> {
        args.get("keys")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .filter(|keys| !keys.is_empty())
            .ok_or_else(|| {
                CommandError::InvalidArgs("expected a non-empty array `keys`".into())
            })
    }
}

impl Guest for KeySimulatorPlugin {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "key-simulator".into(),
            version: "0.1.0".into(),
            authors: vec!["StreamDeck Core".into()],
            dependencies: Vec::<Dependency>::new(),
        }
    }

    fn on_load() {
        host_log(LogLevel::Info, "KeySimulatorPlugin loaded (wasm)");
    }

    fn on_unload() {
        host_log(LogLevel::Info, "KeySimulatorPlugin unloading (wasm)");
    }

    fn interface_ids() -> Vec<String> {
        vec!["KeySimulator".into()]
    }

    fn interface_data() -> Option<String> {
        // Stateless: there is nothing to render between commands.
        None
    }

    fn handle_command(method: String, args_json: String) -> Result<String, CommandError> {
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| CommandError::InvalidArgs(format!("args are not valid JSON: {e}")))?;

        let value = match method.as_str() {
            "simulate_keys" => {
                let keys = Self::arg_keys(&args)?;
                let (mods, main) = Self::split_chord(&keys)?;
                // Errors are reported in the payload rather than as a command
                // error, matching what the native plugin did and what the
                // frontend expects.
                match input::send_hotkey(&mods, &main) {
                    Ok(()) => serde_json::json!({ "ok": true }),
                    Err(e) => serde_json::json!({ "ok": false, "error": e }),
                }
            }

            "send_text" => {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CommandError::InvalidArgs("expected a string `text`".into()))?;
                match input::send_text(text) {
                    Ok(()) => serde_json::json!({ "ok": true }),
                    Err(e) => serde_json::json!({ "ok": false, "error": e }),
                }
            }

            "listen_for_combo" => {
                let timeout_ms = args
                    .get("timeout_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3_000)
                    .min(u32::MAX as u64) as u32;
                match input::record_hotkey(timeout_ms) {
                    Ok(combo) => serde_json::json!({ "combo": combo }),
                    Err(e) => serde_json::json!({ "ok": false, "error": e }),
                }
            }

            "reset_recording" => {
                input::reset_recording();
                serde_json::json!({ "ok": true })
            }

            other => {
                return Err(CommandError::NotFound(format!(
                    "key-simulator has no method '{other}'"
                )))
            }
        };

        serde_json::to_string(&value)
            .map_err(|e| CommandError::Failed(format!("failed to serialize response: {e}")))
    }
}

export!(KeySimulatorPlugin);
