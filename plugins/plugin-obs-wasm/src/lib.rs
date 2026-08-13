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

/// Message for "there is no session". Matched to keep it out of the warn log.
const NOT_CONNECTED: &str = "not connected to OBS";

/// What widgets see. Field names match the native plugin so the frontend was
/// untouched by the port.
/// The status payload. Field names match `ObsStatusResponse` in `sd-api`, so
/// the handler deserializes it directly.
#[derive(Debug, Clone, Default, Serialize)]
struct ObsData {
    connected: bool,
    /// Echoed back so the widget can show what it is (or would be) talking to
    /// even while disconnected.
    host: String,
    port: u16,
    stream_active: bool,
    record_active: bool,
    record_paused: bool,
    virtual_cam_active: bool,
    replay_buffer_active: bool,
    current_scene: String,
    /// Only populated while studio mode is on; empty otherwise.
    preview_scene: String,
    studio_mode: bool,
    /// How long the current stream/recording has been running, in
    /// milliseconds. Zero while the output is not active.
    stream_duration_ms: u64,
    record_duration_ms: u64,
    cpu_usage: f64,
    memory_usage: f64,
    fps: f64,
    /// Not part of the API contract, but useful to widgets and harmless to
    /// callers that ignore unknown fields.
    scenes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

impl ObsData {
    /// Reset everything that describes a live connection, keeping the address
    /// so the widget can still show where it would reconnect to.
    fn clear_live_state(&mut self) {
        self.connected = false;
        self.current_scene.clear();
        self.preview_scene.clear();
        self.scenes.clear();
        self.stream_active = false;
        self.record_active = false;
        self.record_paused = false;
        self.virtual_cam_active = false;
        self.replay_buffer_active = false;
        self.studio_mode = false;
        self.stream_duration_ms = 0;
        self.record_duration_ms = 0;
        self.cpu_usage = 0.0;
        self.memory_usage = 0.0;
        self.fps = 0.0;
    }
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
        // "not connected" is the normal resting state when OBS is closed or
        // the user has not hit Connect, and every widget polls. Logging that
        // at warn buried the real failures in noise.
        let level = if message.contains(NOT_CONNECTED) {
            LogLevel::Debug
        } else {
            LogLevel::Warn
        };
        host_log(level, &message);
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
    fn connect(host: &str, port: u16, password: &str) -> Result<(), String> {
        let url = format!("ws://{host}:{port}");
        let url = url.as_str();
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
            data.host = host.to_string();
            data.port = port;
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
        DATA.with(|d| d.borrow_mut().clear_live_state());
    }

    /// Issue a request and return its response data.
    fn request(
        request_type: &str,
        data: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let (handle, request_id) = SESSION.with(|s| {
            let mut slot = s.borrow_mut();
            let session = slot.as_mut().ok_or(NOT_CONNECTED)?;
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
            DATA.with(|d| {
                let mut data = d.borrow_mut();
                data.stream_active = v["outputActive"].as_bool().unwrap_or(false);
                data.stream_duration_ms = v["outputDuration"].as_u64().unwrap_or(0);
            });
        }

        if let Ok(v) = Self::request("GetRecordStatus", None) {
            DATA.with(|d| {
                let mut data = d.borrow_mut();
                data.record_active = v["outputActive"].as_bool().unwrap_or(false);
                data.record_paused = v["outputPaused"].as_bool().unwrap_or(false);
                data.record_duration_ms = v["outputDuration"].as_u64().unwrap_or(0);
            });
        }

        if let Ok(v) = Self::request("GetVirtualCamStatus", None) {
            DATA.with(|d| {
                d.borrow_mut().virtual_cam_active = v["outputActive"].as_bool().unwrap_or(false)
            });
        }

        if let Ok(v) = Self::request("GetReplayBufferStatus", None) {
            DATA.with(|d| {
                d.borrow_mut().replay_buffer_active = v["outputActive"].as_bool().unwrap_or(false)
            });
        }

        if let Ok(v) = Self::request("GetStudioModeEnabled", None) {
            DATA.with(|d| {
                d.borrow_mut().studio_mode = v["studioModeEnabled"].as_bool().unwrap_or(false)
            });
        }

        // The preview scene only exists while studio mode is on; asking for it
        // otherwise is a guaranteed error per poll, so skip it.
        let studio_on = DATA.with(|d| d.borrow().studio_mode);
        if studio_on {
            if let Ok(v) = Self::request("GetCurrentPreviewScene", None) {
                DATA.with(|d| {
                    d.borrow_mut().preview_scene = v["currentPreviewSceneName"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string()
                });
            }
        } else {
            DATA.with(|d| d.borrow_mut().preview_scene.clear());
        }

        // Stats drive the small readouts in the detailed widget; a server that
        // does not answer simply leaves them at zero.
        if let Ok(v) = Self::request("GetStats", None) {
            DATA.with(|d| {
                let mut data = d.borrow_mut();
                data.cpu_usage = v["cpuUsage"].as_f64().unwrap_or(0.0);
                data.memory_usage = v["memoryUsage"].as_f64().unwrap_or(0.0);
                data.fps = v["activeFps"].as_f64().unwrap_or(0.0);
            });
        }
    }

    /// Split `ws://host:port` back into its parts.
    ///
    /// Only needs to handle what this plugin itself produces plus the obvious
    /// hand-written forms; anything unparseable falls back to the defaults
    /// rather than failing the connect outright.
    fn split_url(url: &str) -> (String, u16) {
        let rest = url
            .strip_prefix("ws://")
            .or_else(|| url.strip_prefix("wss://"))
            .unwrap_or(url);
        let rest = rest.split('/').next().unwrap_or(rest);
        match rest.rsplit_once(':') {
            Some((host, port)) => (
                host.to_string(),
                port.parse().unwrap_or(4455),
            ),
            None => (rest.to_string(), 4455),
        }
    }

    /// Translate OBS's `sceneName`/`sceneIndex` into the `{name, index}` the
    /// API contract uses.
    ///
    /// The wire names are OBS's; the names crossing this boundary are ours.
    /// Doing the mapping here rather than in `sd-api` keeps the host free of
    /// obs-websocket vocabulary — the whole reason the protocol lives in the
    /// guest.
    fn scenes_payload(v: &serde_json::Value) -> serde_json::Value {
        let scenes: Vec<serde_json::Value> = v["scenes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|s| {
                        serde_json::json!({
                            "name": s["sceneName"].as_str().unwrap_or_default(),
                            "index": s["sceneIndex"].as_i64().unwrap_or(0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        serde_json::json!({
            "current_scene": v["currentProgramSceneName"].as_str().unwrap_or_default(),
            "scenes": scenes,
        })
    }

    fn inputs_payload(v: &serde_json::Value) -> serde_json::Value {
        let inputs: Vec<serde_json::Value> = v["inputs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|i| {
                        serde_json::json!({
                            "name": i["inputName"].as_str().unwrap_or_default(),
                            "kind": i["inputKind"].as_str().unwrap_or_default(),
                            "uuid": i["inputUuid"].as_str().unwrap_or_default(),
                            // Filled in by `enrich_inputs` for inputs that
                            // actually carry audio; a color source has no
                            // mixer state to read.
                            "muted": false,
                            "volume": 0.0,
                            "has_audio": false,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        serde_json::Value::Array(inputs)
    }

    /// Read mute and volume for every input that has audio.
    ///
    /// `GetInputList` does not carry mixer state, so this is two extra
    /// requests per input — but only inputs OBS answers for. The mute query
    /// doubles as the probe: it fails for inputs without audio (a color
    /// source, a capture card with no sound), and those keep the defaults and
    /// `has_audio: false` instead of flooding the log with expected errors.
    fn enrich_inputs(inputs: &mut serde_json::Value) {
        let Some(items) = inputs.as_array_mut() else {
            return;
        };

        for item in items {
            let Some(name) = item.get("name").and_then(|v| v.as_str()).map(str::to_string) else {
                continue;
            };

            let mute = Self::request(
                "GetInputMute",
                Some(serde_json::json!({ "inputName": name })),
            );
            let Ok(mute) = mute else { continue };

            let obj = match item.as_object_mut() {
                Some(obj) => obj,
                None => continue,
            };
            obj.insert("has_audio".into(), serde_json::json!(true));
            obj.insert(
                "muted".into(),
                serde_json::json!(mute["inputMuted"].as_bool().unwrap_or(false)),
            );

            if let Ok(volume) = Self::request(
                "GetInputVolume",
                Some(serde_json::json!({ "inputName": name })),
            ) {
                // The multiplier is what OBS's own mixer uses (1.0 = 0 dB).
                // It can exceed 1.0 when the input is boosted; the widget's
                // 0–1 slider clamps the display rather than inventing gain.
                let mul = volume["inputVolumeMul"].as_f64().unwrap_or(0.0);
                obj.insert("volume".into(), serde_json::json!(mul.min(1.0)));
            }
        }
    }

    fn transitions_payload(v: &serde_json::Value) -> serde_json::Value {
        let items: Vec<serde_json::Value> = v["transitions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t["transitionName"].as_str().unwrap_or_default(),
                            "kind": t["transitionKind"].as_str().unwrap_or_default(),
                            "duration": t["transitionDuration"].as_u64().unwrap_or(0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        serde_json::Value::Array(items)
    }

    fn scene_items_payload(v: &serde_json::Value) -> serde_json::Value {
        let items: Vec<serde_json::Value> = v["sceneItems"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|i| {
                        serde_json::json!({
                            "id": i["sceneItemId"].as_i64().unwrap_or(0),
                            "name": i["sourceName"].as_str().unwrap_or_default(),
                            "enabled": i["sceneItemEnabled"].as_bool().unwrap_or(false),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        serde_json::Value::Array(items)
    }

    /// Input kinds OBS can actually play back. Anything else has no transport
    /// controls, so it stays out of the media widget entirely.
    const MEDIA_INPUT_KINDS: &'static [&'static str] =
        &["ffmpeg_source", "media_source", "vlc_source"];

    /// One media input plus its transport state.
    ///
    /// `status` is the (possibly failed) answer to `GetMediaInputStatus`; a
    /// source that cannot report a state still shows up, as stopped, so the
    /// widget lists what is really there.
    fn media_entry(name: &str, kind: &str, status: Result<serde_json::Value, String>) -> serde_json::Value {
        let (state, duration_ms, cursor_ms) = match status {
            Ok(v) => (
                v["mediaState"].as_str().unwrap_or("stopped").to_string(),
                v["mediaDuration"].as_u64().unwrap_or(0),
                v["mediaCursor"].as_u64().unwrap_or(0),
            ),
            Err(_) => ("stopped".to_string(), 0, 0),
        };
        serde_json::json!({
            "name": name,
            "kind": kind,
            "state": state,
            "duration_ms": duration_ms,
            "cursor_ms": cursor_ms,
        })
    }

    fn filters_payload(v: &serde_json::Value) -> serde_json::Value {
        let filters: Vec<serde_json::Value> = v["filters"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|f| {
                        serde_json::json!({
                            "name": f["filterName"].as_str().unwrap_or_default(),
                            "kind": f["filterKind"].as_str().unwrap_or_default(),
                            "enabled": f["filterEnabled"].as_bool().unwrap_or(false),
                            "index": f["filterIndex"].as_i64().unwrap_or(0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        serde_json::Value::Array(filters)
    }

    /// Profiles and scene collections share a shape: the current name plus a
    /// list of `{name}` entries. One reader for both.
    fn named_list_payload(
        v: &serde_json::Value,
        current_field: &str,
        list_field: &str,
    ) -> serde_json::Value {
        let items: Vec<serde_json::Value> = v[list_field]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item["name"].as_str().map(|n| serde_json::json!(n)))
                    .collect()
            })
            .unwrap_or_default();
        serde_json::json!({
            "current": v[current_field].as_str().unwrap_or_default(),
            "items": items,
        })
    }

    fn arg_str(args: &serde_json::Value, key: &str) -> Result<String, CommandError> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| CommandError::InvalidArgs(format!("expected a string `{key}`")))
    }

    /// The widget speaks short verbs; OBS wants the long ones.
    fn media_action_for(action: &str) -> Result<&'static str, CommandError> {
        match action {
            "play" => Ok("MEDIA_INPUT_ACTION_PLAY"),
            "pause" => Ok("MEDIA_INPUT_ACTION_PAUSE"),
            "stop" => Ok("MEDIA_INPUT_ACTION_STOP"),
            "restart" => Ok("MEDIA_INPUT_ACTION_RESTART"),
            "next" => Ok("MEDIA_INPUT_ACTION_NEXT"),
            "previous" => Ok("MEDIA_INPUT_ACTION_PREVIOUS"),
            other => Err(CommandError::InvalidArgs(format!(
                "unknown media action '{other}'"
            ))),
        }
    }

    /// Read the first key that is present, for calls whose argument has been
    /// spelled two ways across the API and the plugin.
    fn arg_str_any(args: &serde_json::Value, keys: &[&str]) -> Result<String, CommandError> {
        keys.iter()
            .find_map(|k| args.get(*k).and_then(|v| v.as_str()))
            .map(str::to_string)
            .ok_or_else(|| {
                CommandError::InvalidArgs(format!("expected a string `{}`", keys[0]))
            })
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
                // The API sends host and port separately; an explicit `url`
                // is still accepted for callers that have one.
                let (host, port) = match args.get("url").and_then(|v| v.as_str()) {
                    Some(url) => Self::split_url(url),
                    None => (
                        args.get("host")
                            .and_then(|v| v.as_str())
                            .filter(|h| !h.is_empty())
                            .unwrap_or("127.0.0.1")
                            .to_string(),
                        args.get("port")
                            .and_then(|v| v.as_u64())
                            .filter(|p| *p > 0 && *p <= u16::MAX as u64)
                            .unwrap_or(4455) as u16,
                    ),
                };
                let password = args
                    .get("password")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                match Self::connect(&host, port, password) {
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

            // Returns the status object *flat*, because `sd-api`
            // deserializes the whole reply straight into `ObsStatusResponse`.
            // Wrapping it in `{ok, status}` made every call fail to parse and
            // surface as "obs plugin not available", which pointed at the
            // wrong thing entirely — the plugin was loaded and answering.
            "get_status" => {
                Self::refresh();
                DATA.with(|d| serde_json::to_value(&*d.borrow()).unwrap_or_default())
            }

            "refresh" => {
                Self::refresh();
                serde_json::json!({ "ok": true })
            }

            "start_stream" => simple!("StartStream"),
            "stop_stream" => simple!("StopStream"),
            "start_record" => simple!("StartRecord"),
            "stop_record" => simple!("StopRecord"),
            "toggle_record_pause" => simple!("ToggleRecordPause"),
            "toggle_virtual_cam" => simple!("ToggleVirtualCam"),
            "start_replay_buffer" => simple!("StartReplayBuffer"),
            "stop_replay_buffer" => simple!("StopReplayBuffer"),
            "save_replay" => simple!("SaveReplayBuffer"),

            "get_scenes" => match Self::request("GetSceneList", None) {
                Ok(v) => Self::scenes_payload(&v),
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
                Ok(v) => {
                    let mut payload = Self::inputs_payload(&v);
                    Self::enrich_inputs(&mut payload);
                    payload
                }
                Err(e) => failed(e),
            },

            "set_input_volume" => {
                let name = Self::arg_str(&args, "input_name")?;
                let volume = args
                    .get("volume")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| CommandError::InvalidArgs("expected a number `volume`".into()))?;
                // The widget slider is a 0–1 fraction, which maps straight to
                // OBS's volume multiplier. Sending it as decibels (the old
                // behaviour) put every input within 1 dB of full volume.
                match Self::request(
                    "SetInputVolume",
                    Some(serde_json::json!({
                        "inputName": name,
                        "inputVolumeMul": volume.max(0.0)
                    })),
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
                Ok(v) => Self::transitions_payload(&v),
                Err(e) => failed(e),
            },

            "set_transition" => {
                // sd-api sends `name`; older callers send `transition_name`.
                let name = Self::arg_str_any(&args, &["name", "transition_name"])?;
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
                    Ok(v) => Self::scene_items_payload(&v),
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

            // A bare boolean: sd-api deserializes this reply straight into
            // `bool`, so an `{ok, enabled}` wrapper never parses.
            "get_studio_mode" => match Self::request("GetStudioModeEnabled", None) {
                Ok(v) => serde_json::json!(v["studioModeEnabled"].as_bool().unwrap_or(false)),
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
                    Ok(_) => {
                        Self::refresh();
                        serde_json::json!({ "ok": true })
                    }
                    Err(e) => failed(e),
                }
            }

            "set_preview_scene" => {
                let name = Self::arg_str(&args, "scene_name")?;
                match Self::request(
                    "SetCurrentPreviewScene",
                    Some(serde_json::json!({ "sceneName": name })),
                ) {
                    Ok(_) => {
                        DATA.with(|d| d.borrow_mut().preview_scene = name);
                        serde_json::json!({ "ok": true })
                    }
                    Err(e) => failed(e),
                }
            }

            "trigger_studio_transition" => simple!("TriggerStudioModeTransition"),

            "get_media_inputs" => match Self::request("GetInputList", None) {
                Ok(v) => {
                    let entries: Vec<serde_json::Value> = v["inputs"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|i| {
                                    let name = i["inputName"].as_str()?.to_string();
                                    let kind = i["inputKind"].as_str().unwrap_or_default();
                                    if !Self::MEDIA_INPUT_KINDS.contains(&kind) {
                                        return None;
                                    }
                                    let status = Self::request(
                                        "GetMediaInputStatus",
                                        Some(serde_json::json!({ "inputName": name })),
                                    );
                                    Some(Self::media_entry(&name, kind, status))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    serde_json::Value::Array(entries)
                }
                Err(e) => failed(e),
            },

            "media_input_action" => {
                let name = Self::arg_str(&args, "input_name")?;
                let action = Self::arg_str(&args, "action")?;
                let media_action = Self::media_action_for(&action)?;
                match Self::request(
                    "TriggerMediaInputAction",
                    Some(serde_json::json!({
                        "inputName": name,
                        "mediaAction": media_action
                    })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            },

            "get_filters" => {
                let source = Self::arg_str(&args, "source_name")?;
                match Self::request(
                    "GetSourceFilterList",
                    Some(serde_json::json!({ "sourceName": source })),
                ) {
                    Ok(v) => Self::filters_payload(&v),
                    Err(e) => failed(e),
                }
            }

            "set_filter_enabled" => {
                let source = Self::arg_str(&args, "source_name")?;
                let filter = Self::arg_str(&args, "filter_name")?;
                let enabled = args.get("enabled").and_then(|v| v.as_bool()).ok_or_else(|| {
                    CommandError::InvalidArgs("expected a boolean `enabled`".into())
                })?;
                match Self::request(
                    "SetSourceFilterEnabled",
                    Some(serde_json::json!({
                        "sourceName": source,
                        "filterName": filter,
                        "filterEnabled": enabled
                    })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "get_profiles" => match Self::request("GetProfileList", None) {
                Ok(v) => Self::named_list_payload(&v, "currentProfileName", "profiles"),
                Err(e) => failed(e),
            },

            "set_profile" => {
                let name = Self::arg_str(&args, "name")?;
                match Self::request(
                    "SetCurrentProfile",
                    Some(serde_json::json!({ "profileName": name })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "get_scene_collections" => match Self::request("GetSceneCollectionList", None) {
                Ok(v) => {
                    Self::named_list_payload(&v, "currentSceneCollectionName", "sceneCollections")
                }
                Err(e) => failed(e),
            },

            "set_scene_collection" => {
                let name = Self::arg_str(&args, "name")?;
                match Self::request(
                    "SetCurrentSceneCollection",
                    Some(serde_json::json!({ "sceneCollectionName": name })),
                ) {
                    Ok(_) => serde_json::json!({ "ok": true }),
                    Err(e) => failed(e),
                }
            }

            "save_screenshot" => {
                // Defaults to the program scene, which is what "take a
                // snapshot" means nine times out of ten.
                let source = match args.get("source_name").and_then(|v| v.as_str()) {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => DATA.with(|d| d.borrow().current_scene.clone()),
                };
                if source.is_empty() {
                    return Err(CommandError::InvalidArgs(
                        "no source to capture: pass `source_name` or connect first".into(),
                    ));
                }
                let format = args
                    .get("format")
                    .and_then(|v| v.as_str())
                    .unwrap_or("png");
                match Self::request(
                    "SaveSourceScreenshot",
                    Some(serde_json::json!({
                        "sourceName": source,
                        "imageFormat": format
                    })),
                ) {
                    // OBS answers with the whole image in base64. The widget
                    // only cares that it landed in OBS's screenshot folder,
                    // and dragging megabytes through the reply buys nothing —
                    // drop it on the guest side of the boundary.
                    Ok(_) => serde_json::json!({ "ok": true, "source": source }),
                    Err(e) => failed(e),
                }
            }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_the_urls_this_plugin_produces() {
        assert_eq!(
            ObsPlugin::split_url("ws://127.0.0.1:4455"),
            ("127.0.0.1".to_string(), 4455)
        );
        assert_eq!(
            ObsPlugin::split_url("wss://obs.example.com:4460"),
            ("obs.example.com".to_string(), 4460)
        );
    }

    /// A missing port or scheme must not fail the connect; it falls back to
    /// the obs-websocket default.
    #[test]
    fn falls_back_to_the_default_port() {
        assert_eq!(
            ObsPlugin::split_url("ws://localhost"),
            ("localhost".to_string(), 4455)
        );
        assert_eq!(
            ObsPlugin::split_url("localhost"),
            ("localhost".to_string(), 4455)
        );
        assert_eq!(
            ObsPlugin::split_url("ws://host:notaport"),
            ("host".to_string(), 4455)
        );
    }

    /// OBS speaks `sceneName`; the API contract says `name`. A drift here is
    /// exactly the class of bug that surfaced as "plugin not available".
    #[test]
    fn translates_obs_scene_names_to_the_api_contract() {
        let obs = serde_json::json!({
            "currentProgramSceneName": "Escena",
            "scenes": [
                {"sceneName": "Escena", "sceneIndex": 0, "sceneUuid": "abc"},
                {"sceneName": "Second", "sceneIndex": 1, "sceneUuid": "def"}
            ]
        });
        let out = ObsPlugin::scenes_payload(&obs);
        assert_eq!(out["current_scene"], "Escena");
        assert_eq!(out["scenes"][0]["name"], "Escena");
        assert_eq!(out["scenes"][0]["index"], 0);
        assert_eq!(out["scenes"][1]["name"], "Second");
        assert_eq!(out["scenes"][1]["index"], 1);
    }

    /// The list endpoints return a bare array, not a wrapper object.
    #[test]
    fn list_payloads_are_arrays() {
        let inputs = ObsPlugin::inputs_payload(&serde_json::json!({
            "inputs": [{"inputName": "Mic", "inputKind": "pulse_input", "inputUuid": "u1"}]
        }));
        assert!(inputs.is_array());
        assert_eq!(inputs[0]["name"], "Mic");
        assert_eq!(inputs[0]["kind"], "pulse_input");
        // Mixer state starts at the defaults; `enrich_inputs` fills it in for
        // inputs that actually carry audio once a session exists.
        assert_eq!(inputs[0]["muted"], false);
        assert_eq!(inputs[0]["volume"], 0.0);
        assert_eq!(inputs[0]["has_audio"], false);

        let transitions = ObsPlugin::transitions_payload(&serde_json::json!({
            "transitions": [{"transitionName": "Fade", "transitionKind": "fade_transition",
                             "transitionDuration": 300}]
        }));
        assert!(transitions.is_array());
        assert_eq!(transitions[0]["name"], "Fade");
        assert_eq!(transitions[0]["duration"], 300);

        let items = ObsPlugin::scene_items_payload(&serde_json::json!({
            "sceneItems": [{"sceneItemId": 7, "sourceName": "Cam", "sceneItemEnabled": true}]
        }));
        assert!(items.is_array());
        assert_eq!(items[0]["id"], 7);
        assert_eq!(items[0]["name"], "Cam");
        assert_eq!(items[0]["enabled"], true);
    }

    /// A media input whose status cannot be read (or one that reports fine)
    /// still shows up, so the widget lists what is really there.
    #[test]
    fn media_entries_fall_back_to_stopped() {
        let ok = ObsPlugin::media_entry(
            "Intro Video",
            "ffmpeg_source",
            Ok(serde_json::json!({
                "mediaState": "playing",
                "mediaDuration": 12000,
                "mediaCursor": 3500
            })),
        );
        assert_eq!(ok["state"], "playing");
        assert_eq!(ok["duration_ms"], 12000);
        assert_eq!(ok["cursor_ms"], 3500);

        let failed =
            ObsPlugin::media_entry("Broken", "media_source", Err("no such input".into()));
        assert_eq!(failed["name"], "Broken");
        assert_eq!(failed["state"], "stopped");
        assert_eq!(failed["duration_ms"], 0);
    }

    #[test]
    fn filters_payload_maps_the_obs_field_names() {
        let filters = ObsPlugin::filters_payload(&serde_json::json!({
            "filters": [
                {"filterName": "Noise Gate", "filterKind": "noise_gate_filter",
                 "filterEnabled": true, "filterIndex": 0},
                {"filterName": "Compressor", "filterKind": "compressor_filter",
                 "filterEnabled": false, "filterIndex": 1}
            ]
        }));
        assert_eq!(filters[0]["name"], "Noise Gate");
        assert_eq!(filters[0]["enabled"], true);
        assert_eq!(filters[1]["enabled"], false);
        assert_eq!(filters[1]["index"], 1);
    }

    #[test]
    fn named_lists_read_the_current_entry_and_the_names() {
        let profiles = ObsPlugin::named_list_payload(
            &serde_json::json!({
                "currentProfileName": "Streaming",
                "profiles": [{"name": "Streaming"}, {"name": "Recording"}]
            }),
            "currentProfileName",
            "profiles",
        );
        assert_eq!(profiles["current"], "Streaming");
        assert_eq!(profiles["items"], serde_json::json!(["Streaming", "Recording"]));

        // A server that omits the list yields an empty array, not null.
        let empty = ObsPlugin::named_list_payload(
            &serde_json::json!({}),
            "currentSceneCollectionName",
            "sceneCollections",
        );
        assert_eq!(empty["current"], "");
        assert_eq!(empty["items"], serde_json::json!([]));
    }

    #[test]
    fn media_actions_translate_to_the_obs_vocabulary() {
        assert_eq!(
            ObsPlugin::media_action_for("play").unwrap(),
            "MEDIA_INPUT_ACTION_PLAY"
        );
        assert_eq!(
            ObsPlugin::media_action_for("previous").unwrap(),
            "MEDIA_INPUT_ACTION_PREVIOUS"
        );
        assert!(ObsPlugin::media_action_for("shuffle").is_err());
    }

    /// A server that omits a list must yield an empty array, not null — the
    /// contract says sequence.
    #[test]
    fn missing_lists_become_empty_arrays() {
        let empty = serde_json::json!({});
        assert_eq!(ObsPlugin::inputs_payload(&empty), serde_json::json!([]));
        assert_eq!(ObsPlugin::transitions_payload(&empty), serde_json::json!([]));
        assert_eq!(ObsPlugin::scene_items_payload(&empty), serde_json::json!([]));
        assert_eq!(ObsPlugin::scenes_payload(&empty)["scenes"], serde_json::json!([]));
    }

    /// sd-api spells the transition argument `name`; the plugin originally
    /// read `transition_name` only, so every call failed as InvalidArgs.
    #[test]
    fn accepts_both_spellings_of_the_transition_argument() {
        let api = serde_json::json!({"name": "Fade"});
        assert_eq!(
            ObsPlugin::arg_str_any(&api, &["name", "transition_name"]).unwrap(),
            "Fade"
        );
        let legacy = serde_json::json!({"transition_name": "Cut"});
        assert_eq!(
            ObsPlugin::arg_str_any(&legacy, &["name", "transition_name"]).unwrap(),
            "Cut"
        );
        assert!(ObsPlugin::arg_str_any(&serde_json::json!({}), &["name"]).is_err());
    }

    #[test]
    fn ignores_a_trailing_path() {
        assert_eq!(
            ObsPlugin::split_url("ws://127.0.0.1:4455/ws"),
            ("127.0.0.1".to_string(), 4455)
        );
    }
}
