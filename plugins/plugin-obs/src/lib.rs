use plugin_system::{Plugin, PluginContext, PluginMetadata};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

mod obs_controller;
use obs_controller::ObsController;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsData {
    pub connected: bool,
    pub host: String,
    pub port: u16,
    pub stream_active: bool,
    pub record_active: bool,
    pub record_paused: bool,
    pub virtual_cam_active: bool,
    pub replay_buffer_active: bool,
    pub current_scene: String,
    pub studio_mode: bool,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub fps: f64,
}

pub struct ObsPlugin {
    controller: ObsController,
    runtime: Runtime,
    data: ObsData,
}

impl Default for ObsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ObsPlugin {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        Self {
            controller: ObsController::new(),
            runtime,
            data: ObsData::default(),
        }
    }

    pub fn interface_ids(&self) -> Vec<&'static str> {
        vec!["ObsControl"]
    }

    pub fn interface_data(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.data).ok()
    }

    pub fn handle_command(
        &mut self,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        match method {
            "connect" => {
                let host = args
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("127.0.0.1");
                let port = args.get("port").and_then(|v| v.as_u64()).unwrap_or(4455) as u16;
                let password = args.get("password").and_then(|v| v.as_str());

                match self
                    .runtime
                    .block_on(self.controller.connect(host, port, password))
                {
                    Ok(()) => {
                        self.refresh_status();
                        Some(serde_json::json!({"ok": true}))
                    }
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "disconnect" => {
                self.runtime.block_on(self.controller.disconnect());
                self.data = ObsData::default();
                Some(serde_json::json!({"ok": true}))
            }
            "refresh" => {
                self.refresh_status();
                Some(serde_json::json!({"ok": true}))
            }
            "get_status" => {
                self.refresh_status();
                Some(serde_json::to_value(&self.data).unwrap_or_default())
            }
            "start_stream" => match self.runtime.block_on(self.controller.start_stream()) {
                Ok(()) => Some(serde_json::json!({"ok": true})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "stop_stream" => match self.runtime.block_on(self.controller.stop_stream()) {
                Ok(()) => Some(serde_json::json!({"ok": true})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "start_record" => match self.runtime.block_on(self.controller.start_record()) {
                Ok(()) => Some(serde_json::json!({"ok": true})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "stop_record" => match self.runtime.block_on(self.controller.stop_record()) {
                Ok(_) => Some(serde_json::json!({"ok": true})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "toggle_record_pause" => {
                match self.runtime.block_on(self.controller.toggle_record_pause()) {
                    Ok(_) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "get_scenes" => match self.runtime.block_on(self.controller.get_scene_list()) {
                Ok((current, scenes)) => Some(serde_json::json!({
                    "current_scene": current,
                    "scenes": scenes
                })),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "set_scene" => {
                let scene_name = args
                    .get("scene_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match self
                    .runtime
                    .block_on(self.controller.set_current_scene(scene_name))
                {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "get_inputs" => match self.runtime.block_on(self.controller.get_input_list()) {
                Ok(inputs) => Some(serde_json::json!({"inputs": inputs})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "set_input_volume" => {
                let input_name = args
                    .get("input_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let volume = args.get("volume").and_then(|v| v.as_f64()).unwrap_or(1.0);
                match self
                    .runtime
                    .block_on(self.controller.set_input_volume(input_name, volume))
                {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "set_input_mute" => {
                let input_name = args
                    .get("input_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let muted = args.get("muted").and_then(|v| v.as_bool()).unwrap_or(false);
                match self
                    .runtime
                    .block_on(self.controller.set_input_mute(input_name, muted))
                {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "toggle_virtual_cam" => {
                match self.runtime.block_on(self.controller.toggle_virtual_cam()) {
                    Ok(_) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "save_replay" => match self.runtime.block_on(self.controller.save_replay_buffer()) {
                Ok(()) => Some(serde_json::json!({"ok": true})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "get_transitions" => match self.runtime.block_on(self.controller.get_transitions()) {
                Ok(transitions) => Some(serde_json::json!({"transitions": transitions})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "set_transition" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                match self.runtime.block_on(self.controller.set_transition(name)) {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "get_scene_items" => {
                let scene_name = args
                    .get("scene_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match self
                    .runtime
                    .block_on(self.controller.get_scene_item_list(scene_name))
                {
                    Ok(items) => Some(serde_json::json!({"items": items})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "set_scene_item_enabled" => {
                let scene_name = args
                    .get("scene_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let item_id = args.get("item_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let enabled = args
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                match self.runtime.block_on(
                    self.controller
                        .set_scene_item_enabled(scene_name, item_id, enabled),
                ) {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "get_studio_mode" => match self.runtime.block_on(self.controller.get_studio_mode()) {
                Ok(enabled) => Some(serde_json::json!({"enabled": enabled})),
                Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
            },
            "set_studio_mode" => {
                let enabled = args
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                match self
                    .runtime
                    .block_on(self.controller.set_studio_mode(enabled))
                {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            _ => None,
        }
    }

    fn refresh_status(&mut self) {
        if !self.controller.is_connected() {
            self.data = ObsData::default();
            return;
        }

        let stream = self
            .runtime
            .block_on(self.controller.get_stream_status())
            .unwrap_or_default();
        let record = self
            .runtime
            .block_on(self.controller.get_record_status())
            .unwrap_or_default();
        let virtual_cam = self
            .runtime
            .block_on(self.controller.get_virtual_cam_status())
            .unwrap_or(false);
        let replay_buffer = self
            .runtime
            .block_on(self.controller.get_replay_buffer_status())
            .unwrap_or(false);
        let current_scene = self
            .runtime
            .block_on(self.controller.get_scene_list())
            .map(|(s, _)| s)
            .unwrap_or_default();
        let studio_mode = self
            .runtime
            .block_on(self.controller.get_studio_mode())
            .unwrap_or(false);
        let stats = self.runtime.block_on(self.controller.get_stats()).ok();

        let conn = self.controller.connection_info();
        self.data = ObsData {
            connected: true,
            host: conn.host.clone(),
            port: conn.port,
            stream_active: stream.active,
            record_active: record.active,
            record_paused: record.paused,
            virtual_cam_active: virtual_cam,
            replay_buffer_active: replay_buffer,
            current_scene,
            studio_mode,
            cpu_usage: stats.as_ref().map(|s| s.cpu_usage).unwrap_or(0.0),
            memory_usage: stats.as_ref().map(|s| s.memory_usage).unwrap_or(0.0),
            fps: stats.as_ref().map(|s| s.fps).unwrap_or(0.0),
        };
    }
}

#[plugin_system::plugin_export]
impl Plugin for ObsPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "obs",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        log::info!("OBS Plugin loaded");
    }

    fn on_unload(&mut self) {
        log::info!("OBS Plugin unloading");
        self.runtime.block_on(self.controller.disconnect());
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
