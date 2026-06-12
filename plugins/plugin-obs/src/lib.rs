use plugin_system::{command, CommandResult, Plugin, PluginContext, PluginMetadata};
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

    #[command("connect")]
    fn obs_connect(&mut self, host: String, port: u16, password: Option<String>) -> CommandResult {
        let password_str = password.as_deref();
        match self.runtime.block_on(self.controller.connect(&host, port, password_str)) {
            Ok(()) => {
                self.refresh_status();
                Ok(serde_json::json!({"ok": true}))
            }
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("disconnect")]
    fn obs_disconnect(&mut self) -> CommandResult {
        self.runtime.block_on(self.controller.disconnect());
        self.data = ObsData::default();
        Ok(serde_json::json!({"ok": true}))
    }

    #[command("refresh")]
    fn obs_refresh(&mut self) -> CommandResult {
        self.refresh_status();
        Ok(serde_json::json!({"ok": true}))
    }

    #[command("get_status")]
    fn obs_get_status(&mut self) -> CommandResult {
        self.refresh_status();
        Ok(serde_json::to_value(&self.data).unwrap_or_default())
    }

    #[command("start_stream")]
    fn obs_start_stream(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.start_stream()) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("stop_stream")]
    fn obs_stop_stream(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.stop_stream()) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("start_record")]
    fn obs_start_record(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.start_record()) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("stop_record")]
    fn obs_stop_record(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.stop_record()) {
            Ok(_) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("toggle_record_pause")]
    fn obs_toggle_record_pause(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.toggle_record_pause()) {
            Ok(_) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("get_scenes")]
    fn obs_get_scenes(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.get_scene_list()) {
            Ok((current, scenes)) => Ok(serde_json::json!({
                "current_scene": current,
                "scenes": scenes
            })),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_scene")]
    fn obs_set_scene(&mut self, scene_name: String) -> CommandResult {
        match self.runtime.block_on(self.controller.set_current_scene(&scene_name)) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("get_inputs")]
    fn obs_get_inputs(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.get_input_list()) {
            Ok(inputs) => Ok(serde_json::json!({"inputs": inputs})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_input_volume")]
    fn obs_set_input_volume(&mut self, input_name: String, volume: f64) -> CommandResult {
        match self.runtime.block_on(self.controller.set_input_volume(&input_name, volume)) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_input_mute")]
    fn obs_set_input_mute(&mut self, input_name: String, muted: bool) -> CommandResult {
        match self.runtime.block_on(self.controller.set_input_mute(&input_name, muted)) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("toggle_virtual_cam")]
    fn obs_toggle_virtual_cam(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.toggle_virtual_cam()) {
            Ok(_) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("save_replay")]
    fn obs_save_replay(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.save_replay_buffer()) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("get_transitions")]
    fn obs_get_transitions(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.get_transitions()) {
            Ok(transitions) => Ok(serde_json::json!({"transitions": transitions})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_transition")]
    fn obs_set_transition(&mut self, name: String) -> CommandResult {
        match self.runtime.block_on(self.controller.set_transition(&name)) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("get_scene_items")]
    fn obs_get_scene_items(&mut self, scene_name: String) -> CommandResult {
        match self.runtime.block_on(self.controller.get_scene_item_list(&scene_name)) {
            Ok(items) => Ok(serde_json::json!({"items": items})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_scene_item_enabled")]
    fn obs_set_scene_item_enabled(&mut self, scene_name: String, item_id: i32, enabled: bool) -> CommandResult {
        match self.runtime.block_on(
            self.controller
                .set_scene_item_enabled(&scene_name, item_id, enabled),
        ) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("get_studio_mode")]
    fn obs_get_studio_mode(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.get_studio_mode()) {
            Ok(enabled) => Ok(serde_json::json!({"enabled": enabled})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_studio_mode")]
    fn obs_set_studio_mode(&mut self, enabled: bool) -> CommandResult {
        match self.runtime.block_on(self.controller.set_studio_mode(enabled)) {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
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

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["ObsControl"]
    }

    fn interface_data(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.data).ok()
    }

    fn handle_command(
        &mut self,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        match method {
            "connect" => {
                let host = args.get("host").and_then(|v| v.as_str()).unwrap_or("127.0.0.1").to_string();
                let port = args.get("port").and_then(|v| v.as_u64()).unwrap_or(4455) as u16;
                let password = args.get("password").and_then(|v| v.as_str()).map(String::from);
                plugin_system::command_to_json(self.obs_connect(host, port, password))
            }
            "disconnect" => plugin_system::command_to_json(self.obs_disconnect()),
            "refresh" => plugin_system::command_to_json(self.obs_refresh()),
            "get_status" => plugin_system::command_to_json(self.obs_get_status()),
            "start_stream" => plugin_system::command_to_json(self.obs_start_stream()),
            "stop_stream" => plugin_system::command_to_json(self.obs_stop_stream()),
            "start_record" => plugin_system::command_to_json(self.obs_start_record()),
            "stop_record" => plugin_system::command_to_json(self.obs_stop_record()),
            "toggle_record_pause" => plugin_system::command_to_json(self.obs_toggle_record_pause()),
            "get_scenes" => plugin_system::command_to_json(self.obs_get_scenes()),
            "set_scene" => {
                let scene_name = args.get("scene_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                plugin_system::command_to_json(self.obs_set_scene(scene_name))
            }
            "get_inputs" => plugin_system::command_to_json(self.obs_get_inputs()),
            "set_input_volume" => {
                let input_name = args.get("input_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let volume = args.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);
                plugin_system::command_to_json(self.obs_set_input_volume(input_name, volume))
            }
            "set_input_mute" => {
                let input_name = args.get("input_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let muted = args.get("muted").and_then(|v| v.as_bool()).unwrap_or(false);
                plugin_system::command_to_json(self.obs_set_input_mute(input_name, muted))
            }
            "toggle_virtual_cam" => plugin_system::command_to_json(self.obs_toggle_virtual_cam()),
            "save_replay" => plugin_system::command_to_json(self.obs_save_replay()),
            "get_transitions" => plugin_system::command_to_json(self.obs_get_transitions()),
            "set_transition" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                plugin_system::command_to_json(self.obs_set_transition(name))
            }
            "get_scene_items" => {
                let scene_name = args.get("scene_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                plugin_system::command_to_json(self.obs_get_scene_items(scene_name))
            }
            "set_scene_item_enabled" => {
                let scene_name = args.get("scene_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let item_id = args.get("item_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                plugin_system::command_to_json(self.obs_set_scene_item_enabled(scene_name, item_id, enabled))
            }
            "get_studio_mode" => plugin_system::command_to_json(self.obs_get_studio_mode()),
            "set_studio_mode" => {
                let enabled = args.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                plugin_system::command_to_json(self.obs_set_studio_mode(enabled))
            }
            _ => None,
        }
    }
}
