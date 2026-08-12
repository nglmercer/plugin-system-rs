//! OBS Studio control over obs-websocket 5.x, as a WASI Preview 2 component.
//!
//! Of the four ported plugins this is the one that stayed substantial. The
//! host grants a plain `websocket`; the identify handshake, request
//! correlation and the whole OBS command surface live here. That was the
//! deliberate choice — wrapping `obws` host-side would have been less code but
//! would have moved the interesting part out of the sandbox.
//!
//! `obws` and its tokio runtime are gone. Components are single-threaded and
//! the host capability is synchronous, so requests are written and their
//! responses read back inline. For a plugin whose calls all originate from an
//! HTTP request or a widget poll, that is simpler than what it replaced.

wit_bindgen::generate!({
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

mod protocol;

use std::cell::RefCell;

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::host::log as host_log;
use streamdeck::plugin::types::{CommandError, Dependency, LogLevel, Metadata};
use streamdeck::plugin::websocket as ws;

use protocol::{op, parse_response, Response};
use serde::Serialize;

/// How long to wait for any single response frame.
const RESPONSE_TIMEOUT_MS: u32 = 5_000;

/// Cap on frames read while looking for a matching response, so an OBS server
/// streaming events cannot keep us reading forever.
const MAX_FRAMES_PER_REQUEST: usize = 32;

/// What widgets see. Field names match the native plugin so the frontend was
/// untouched by the port.
#[derive(Debug, Clone, Default, Serialize)]
struct ObsData {
    connected: bool,
    url: String,
    current_scene: String,
    scenes: Vec<String>,
    streaming: bool,
    recording: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

struct Session {
    handle: u32,
    next_request_id: u64,
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    static DATA: RefCell<ObsData> = RefCell::new(ObsData::default());
}

struct ObsPlugin;

impl ObsPlugin {
    fn set_error(message: impl Into<String>) {
        let message = message.into();
        host_log(LogLevel::Warn, &message);
        DATA.with(|d| d.borrow_mut().last_error = Some(message));
    }

    /// Read frames until one satisfies `want`, or the budget runs out.
    ///
    /// Events and responses to other requests are skipped rather than treated
    /// as errors: the protocol allows them to be interleaved.
    fn read_until(
        handle: u32,
        mut want: impl FnMut(&serde_json::Value) -> bool,
    ) -> Result<serde_json::Value, String> {
        for _ in 0..MAX_FRAMES_PER_REQUEST {
            let raw = match ws::receive(handle, RESPONSE_TIMEOUT_MS)? {
                Some(raw) => raw,
                None => return Err("timed out waiting for an OBS response".into()),
            };

            let frame: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| format!("OBS sent a frame that is not JSON: {e}"))?;

            if want(&frame) {
                return Ok(frame);
            }

            if frame["op"].as_u64() == Some(op::EVENT) {
                continue;
            }
        }
        Err("gave up waiting for a matching OBS response".into())
    }

    /// Open a connection and complete the identify handshake.
    fn connect(url: &str, password: &str) -> Result<(), String> {
        // Drop any previous session first, or a reconnect leaks a socket.
        Self::disconnect();

        let handle = ws::connect(url)?;

        let hello = Self::read_until(handle, |f| f["op"].as_u64() == Some(op::HELLO))
            .map_err(|e| {
                let _ = ws::close(handle);
                e
            })?;

        let identify = protocol::identify_message(&hello["d"], password);
        ws::send(handle, &identify.to_string()).map_err(|e| {
            let _ = ws::close(handle);
            e
        })?;

        Self::read_until(handle, |f| f["op"].as_u64() == Some(op::IDENTIFIED)).map_err(|e| {
            let _ = ws::close(handle);
            // The server closes the socket on a bad password rather than
            // replying, so a read failure here almost always means auth.
            format!("OBS did not accept the connection (check the password): {e}")
        })?;

        SESSION.with(|s| {
            *s.borrow_mut() = Some(Session {
                handle,
                next_request_id: 1,
            })
        });

        DATA.with(|d| {
            let mut data = d.borrow_mut();
            data.connected = true;
            data.url = url.to_string();
            data.last_error = None;
        });

        host_log(LogLevel::Info, &format!("connected to OBS at {url}"));
        Ok(())
    }

    fn disconnect() {
        SESSION.with(|s| {
            if let Some(session) = s.borrow_mut().take() {
                let _ = ws::close(session.handle);
            }
        });
        DATA.with(|d| {
            let mut data = d.borrow_mut();
            data.connected = false;
            data.current_scene.clear();
            data.scenes.clear();
            data.streaming = false;
            data.recording = false;
        });
    }

    /// Issue a request and return its response data.
    fn request(
        request_type: &str,
        data: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let (handle, request_id) = SESSION.with(|s| {
            let mut slot = s.borrow_mut();
            let session = slot.as_mut().ok_or("not connected to OBS")?;
            let id = session.next_request_id;
            session.next_request_id += 1;
            Ok::<_, String>((session.handle, format!("sd-{id}")))
        })?;

        let msg = protocol::request_message(request_type, &request_id, data);
        ws::send(handle, &msg.to_string())?;

        // Match on the request id, not just the opcode: a response to an
        // earlier timed-out request may still be in flight.
        let frame = Self::read_until(handle, |f| {
            f["op"].as_u64() == Some(op::REQUEST_RESPONSE)
                && f["d"]["requestId"].as_str() == Some(request_id.as_str())
        })?;

        match parse_response(&frame["d"]) {
            Response::Ok(data) => Ok(data),
            Response::Failed(e) => Err(format!("{request_type} failed: {e}")),
        }
    }

    /// Re-read the state widgets display.
    ///
    /// Individual failures are tolerated: a server that answers scenes but not
    /// stream status should still populate what it can.
    fn refresh() {
        if !SESSION.with(|s| s.borrow().is_some()) {
            return;
        }

        if let Ok(v) = Self::request("GetSceneList", None) {
            DATA.with(|d| {
                let mut data = d.borrow_mut();
                data.current_scene = v["currentProgramSceneName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                data.scenes = v["scenes"]
                    .as_array()
                    .map(|scenes| {
                        scenes
                            .iter()
                            .filter_map(|s| s["sceneName"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
            });
        }

        if let Ok(v) = Self::request("GetStreamStatus", None) {
            DATA.with(|d| d.borrow_mut().streaming = v["outputActive"].as_bool().unwrap_or(false));
        }

        if let Ok(v) = Self::request("GetRecordStatus", None) {
            DATA.with(|d| d.borrow_mut().recording = v["outputActive"].as_bool().unwrap_or(false));
        }
    }

    fn arg_str(args: &serde_json::Value, key: &str) -> Result<String, CommandError> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| CommandError::InvalidArgs(format!("expected a string `{key}`")))
    }
}

impl Guest for ObsPlugin {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "obs".into(),
            version: "0.1.0".into(),
            authors: vec!["StreamDeck Core".into()],
            dependencies: Vec::<Dependency>::new(),
        }
    }

    fn on_load() {
        host_log(LogLevel::Info, "ObsPlugin loaded (wasm)");
    }

    fn on_unload() {
        Self::disconnect();
        host_log(LogLevel::Info, "ObsPlugin unloading (wasm)");
    }

    fn interface_ids() -> Vec<String> {
        vec!["ObsControl".into()]
    }

    fn interface_data() -> Option<String> {
        DATA.with(|d| serde_json::to_string(&*d.borrow()).ok())
    }

    fn handle_command(method: String, args_json: String) -> Result<String, CommandError> {
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| CommandError::InvalidArgs(format!("args are not valid JSON: {e}")))?;

        // Every arm reports failure in the payload rather than as a command
        // error: OBS being unreachable is an expected condition the widget
        // renders, not a malformed call.
        let failed = |e: String| {
            ObsPlugin::set_error(e.clone());
            serde_json::json!({ "ok": false, "error": e })
        };

        /// Run a request that returns no interesting data.
        macro_rules! simple {
            ($ty:expr) => {
                match Self::request($ty, None) {
                    Ok(_) => {
                        Self::refresh();
                        serde_json::json!({ "ok": true })
                    }
                    Err(e) => failed(e),
                }
            };
        }

        let value = match method.as_str() {
            "connect" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| "ws://localhost:4455".to_string());
                let password = args
                    .get("password")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                match Self::connect(&url, password) {
                    Ok(()) => {
                        Self::refresh();
                        serde_json::json!({ "ok": true })
                    }
                    Err(e) => failed(e),
                }
            }

            "disconnect" => {
                Self::disconnect();
                serde_json::json!({ "ok": true })
            }

            "refresh" | "get_status" => {
                Self::refresh();
                DATA.with(|d| serde_json::json!({ "ok": true, "status": &*d.borrow() }))
            }

            "start_stream" => simple!("StartStream"),
            "stop_stream" => simple!("StopStream"),
            "start_record" => simple!("StartRecord"),
            "stop_record" => simple!("StopRecord"),
            "toggle_record_pause" => simple!("ToggleRecordPause"),
            "toggle_virtual_cam" => simple!("ToggleVirtualCam"),
            "save_replay" => simple!("SaveReplayBuffer"),

            "get_scenes" => match Self::request("GetSceneList", None) {
                Ok(v) => serde_json::json!({ "ok": true, "scenes": v["scenes"],
                                             "current": v["currentProgramSceneName"] }),
                Err(e) => failed(e),
            },

            "set_scene" => {
                let name = Self::arg_str(&args, "scene_name")?;
                match Self::request(
                    "SetCurrentProgramScene",
                    Some(serde_json::json!({ "sceneName": name })),
                ) {
                    Ok(_) => {
                        DATA.with(|d| d.borrow_mut().current_scene = name);
                        serde_json::json!({ "ok": true })
                    }
                    Err(e) => failed(e),
                }
            }

            "get_inputs" => match Self::request("GetInputList", None) {
                Ok(v) => serde_json::json!({ "ok": true, "inputs": v["inputs"] }),
                Err(e) => failed(e),
            },

            "set_input_volume" => {
                let name = Self::arg_str(&args, "input_name")?;
                let volume = args
                    .get("volume")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| CommandError::InvalidArgs("expected a number `volume`".into()))?;
                match Self::request(
                    "SetInputVolume",
                    Some(serde_json::json!({ "inputName": name, "inputVolumeDb": volume })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "set_input_mute" => {
                let name = Self::arg_str(&args, "input_name")?;
                let muted = args
                    .get("muted")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| CommandError::InvalidArgs("expected a boolean `muted`".into()))?;
                match Self::request(
                    "SetInputMute",
                    Some(serde_json::json!({ "inputName": name, "inputMuted": muted })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "get_transitions" => match Self::request("GetSceneTransitionList", None) {
                Ok(v) => serde_json::json!({ "ok": true, "transitions": v["transitions"] }),
                Err(e) => failed(e),
            },

            "set_transition" => {
                let name = Self::arg_str(&args, "transition_name")?;
                match Self::request(
                    "SetCurrentSceneTransition",
                    Some(serde_json::json!({ "transitionName": name })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "get_scene_items" => {
                let scene = Self::arg_str(&args, "scene_name")?;
                match Self::request(
                    "GetSceneItemList",
                    Some(serde_json::json!({ "sceneName": scene })),
                ) {
                    Ok(v) => serde_json::json!({ "ok": true, "items": v["sceneItems"] }),
                    Err(e) => failed(e),
                }
            }

            "set_scene_item_enabled" => {
                let scene = Self::arg_str(&args, "scene_name")?;
                let item_id = args.get("item_id").and_then(|v| v.as_i64()).ok_or_else(|| {
                    CommandError::InvalidArgs("expected an integer `item_id`".into())
                })?;
                let enabled = args.get("enabled").and_then(|v| v.as_bool()).ok_or_else(|| {
                    CommandError::InvalidArgs("expected a boolean `enabled`".into())
                })?;
                match Self::request(
                    "SetSceneItemEnabled",
                    Some(serde_json::json!({
                        "sceneName": scene,
                        "sceneItemId": item_id,
                        "sceneItemEnabled": enabled
                    })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "get_studio_mode" => match Self::request("GetStudioModeEnabled", None) {
                Ok(v) => serde_json::json!({ "ok": true, "enabled": v["studioModeEnabled"] }),
                Err(e) => failed(e),
            },

            "set_studio_mode" => {
                let enabled = args.get("enabled").and_then(|v| v.as_bool()).ok_or_else(|| {
                    CommandError::InvalidArgs("expected a boolean `enabled`".into())
                })?;
                match Self::request(
                    "SetStudioModeEnabled",
                    Some(serde_json::json!({ "studioModeEnabled": enabled })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "get_stats" => match Self::request("GetStats", None) {
                Ok(v) => serde_json::json!({ "ok": true, "stats": v }),
                Err(e) => failed(e),
            },

            other => {
                return Err(CommandError::NotFound(format!(
                    "obs has no method '{other}'"
                )))
            }
        };

        serde_json::to_string(&value)
            .map_err(|e| CommandError::Failed(format!("failed to serialize response: {e}")))
    }
}

export!(ObsPlugin);
