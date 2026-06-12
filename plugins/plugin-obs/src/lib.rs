use plugin_system::{command, CommandResult, PluginContext, PluginMetadata};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

mod obs_controller;
use obs_controller::ObsController;
#[cfg(test)]
use obs_controller::{ObsInput, ObsScene, ObsSceneItem, ObsTestState, ObsTransition};

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
    #[cfg(test)]
    pub(crate) fn with_test_controller(state: ObsTestState) -> Self {
        Self {
            controller: ObsController::with_test_state(state),
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
            data: ObsData::default(),
        }
    }
}

#[plugin_system::plugin_export(interfaces = ["ObsControl"])]
impl ObsPlugin {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        Self {
            controller: ObsController::new(),
            runtime,
            data: ObsData::default(),
        }
    }

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

    fn interface_data(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.data).ok()
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
        match self
            .runtime
            .block_on(self.controller.connect(&host, port, password_str))
        {
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
        match self
            .runtime
            .block_on(self.controller.set_current_scene(&scene_name))
        {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("get_inputs")]
    fn obs_get_inputs(&mut self) -> CommandResult {
        match self.runtime.block_on(self.controller.get_input_list()) {
            Ok(inputs) => serde_json::to_value(inputs).map_err(|e| e.to_string()),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_input_volume")]
    fn obs_set_input_volume(&mut self, input_name: String, volume: f64) -> CommandResult {
        match self
            .runtime
            .block_on(self.controller.set_input_volume(&input_name, volume))
        {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_input_mute")]
    fn obs_set_input_mute(&mut self, input_name: String, muted: bool) -> CommandResult {
        match self
            .runtime
            .block_on(self.controller.set_input_mute(&input_name, muted))
        {
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
            Ok(transitions) => serde_json::to_value(transitions).map_err(|e| e.to_string()),
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
        match self
            .runtime
            .block_on(self.controller.get_scene_item_list(&scene_name))
        {
            Ok(items) => serde_json::to_value(items).map_err(|e| e.to_string()),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_scene_item_enabled")]
    fn obs_set_scene_item_enabled(
        &mut self,
        scene_name: String,
        item_id: i32,
        enabled: bool,
    ) -> CommandResult {
        match self
            .runtime
            .block_on(
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
            Ok(enabled) => Ok(serde_json::json!(enabled)),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }

    #[command("set_studio_mode")]
    fn obs_set_studio_mode(&mut self, enabled: bool) -> CommandResult {
        match self
            .runtime
            .block_on(self.controller.set_studio_mode(enabled))
        {
            Ok(()) => Ok(serde_json::json!({"ok": true})),
            Err(e) => Ok(serde_json::json!({"ok": false, "error": e})),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_system::Plugin;

    fn test_state() -> ObsTestState {
        ObsTestState {
            current_scene: "Escena".to_string(),
            scenes: vec![
                ObsScene {
                    name: "Escena".to_string(),
                    index: 0,
                },
                ObsScene {
                    name: "Escena 2".to_string(),
                    index: 1,
                },
            ],
            inputs: vec![ObsInput {
                name: "Mic".to_string(),
                kind: "audio_input_capture".to_string(),
                uuid: "input-uuid".to_string(),
                muted: false,
                volume: 0.5,
            }],
            transitions: vec![
                ObsTransition {
                    name: "Cut".to_string(),
                    kind: "cut".to_string(),
                    duration: 0,
                },
                ObsTransition {
                    name: "Fade".to_string(),
                    kind: "fade".to_string(),
                    duration: 0,
                },
            ],
            scene_items: vec![ObsSceneItem {
                id: 1,
                name: "Camera".to_string(),
                enabled: true,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn metadata_and_interface_ids_are_generated() {
        let plugin = ObsPlugin::with_test_controller(test_state());

        assert_eq!(plugin.metadata().name, "obs");
        assert_eq!(plugin.interface_ids(), vec!["ObsControl"]);
    }

    #[test]
    fn connect_and_status_commands_return_canned_obs_data() {
        let mut plugin = ObsPlugin::with_test_controller(test_state());

        let connected = plugin
            .handle_command(
                "connect",
                serde_json::json!({"host": "127.0.0.1", "port": 4455, "password": null}),
            )
            .unwrap();
        let status = plugin
            .handle_command("get_status", serde_json::json!({}))
            .unwrap();

        assert_eq!(connected["ok"], true);
        assert_eq!(status["connected"], true);
        assert_eq!(status["host"], "127.0.0.1");
        assert_eq!(status["port"], 4455);
        assert_eq!(status["current_scene"], "Escena");
        assert_eq!(status["cpu_usage"], 12.5);
    }

    #[test]
    fn scene_and_input_commands_use_mock_controller() {
        let mut plugin = ObsPlugin::with_test_controller(test_state());
        plugin
            .handle_command(
                "connect",
                serde_json::json!({"host": "127.0.0.1", "port": 4455, "password": null}),
            )
            .unwrap();

        let scenes = plugin
            .handle_command("get_scenes", serde_json::json!({}))
            .unwrap();
        let inputs = plugin
            .handle_command("get_inputs", serde_json::json!({}))
            .unwrap();

        assert_eq!(scenes["current_scene"], "Escena");
        assert_eq!(scenes["scenes"][1]["name"], "Escena 2");
        assert_eq!(inputs[0]["name"], "Mic");
        assert_eq!(inputs[0]["volume"], 0.5);

        plugin
            .handle_command("set_scene", serde_json::json!({"scene_name": "Escena 2"}))
            .unwrap();
        plugin
            .handle_command(
                "set_input_volume",
                serde_json::json!({"input_name": "Mic", "volume": 0.75}),
            )
            .unwrap();
        plugin
            .handle_command(
                "set_input_mute",
                serde_json::json!({"input_name": "Mic", "muted": true}),
            )
            .unwrap();

        let status = plugin
            .handle_command("get_status", serde_json::json!({}))
            .unwrap();
        let inputs = plugin
            .handle_command("get_inputs", serde_json::json!({}))
            .unwrap();
        assert_eq!(status["current_scene"], "Escena 2");
        assert_eq!(inputs[0]["volume"], 0.75);
        assert!(inputs[0]["muted"].as_bool().unwrap());
    }

    #[test]
    fn streaming_recording_and_studio_commands_use_mock_controller() {
        let mut plugin = ObsPlugin::with_test_controller(test_state());
        plugin
            .handle_command(
                "connect",
                serde_json::json!({"host": "127.0.0.1", "port": 4455, "password": null}),
            )
            .unwrap();

        assert_eq!(
            plugin
                .handle_command("start_stream", serde_json::json!({}))
                .unwrap()["ok"],
            true
        );
        assert_eq!(
            plugin
                .handle_command("start_record", serde_json::json!({}))
                .unwrap()["ok"],
            true
        );
        assert_eq!(
            plugin
                .handle_command("toggle_record_pause", serde_json::json!({}))
                .unwrap()["ok"],
            true
        );
        assert_eq!(
            plugin
                .handle_command("toggle_virtual_cam", serde_json::json!({}))
                .unwrap()["ok"],
            true
        );
        assert_eq!(
            plugin
                .handle_command("set_studio_mode", serde_json::json!({"enabled": true}))
                .unwrap()["ok"],
            true
        );

        let status = plugin
            .handle_command("get_status", serde_json::json!({}))
            .unwrap();
        assert!(status["stream_active"].as_bool().unwrap());
        assert!(status["record_active"].as_bool().unwrap());
        assert!(status["record_paused"].as_bool().unwrap());
        assert!(status["virtual_cam_active"].as_bool().unwrap());
        assert!(status["studio_mode"].as_bool().unwrap());
    }

    #[test]
    fn transitions_scene_items_and_replay_commands_use_mock_controller() {
        let mut plugin = ObsPlugin::with_test_controller(test_state());
        plugin
            .handle_command(
                "connect",
                serde_json::json!({"host": "127.0.0.1", "port": 4455, "password": null}),
            )
            .unwrap();

        let transitions = plugin
            .handle_command("get_transitions", serde_json::json!({}))
            .unwrap();
        let items = plugin
            .handle_command(
                "get_scene_items",
                serde_json::json!({"scene_name": "Escena"}),
            )
            .unwrap();
        assert_eq!(transitions[0]["name"], "Cut");
        assert_eq!(items[0]["name"], "Camera");
        assert!(items[0]["enabled"].as_bool().unwrap());

        assert_eq!(
            plugin
                .handle_command("save_replay", serde_json::json!({}))
                .unwrap()["ok"],
            true
        );
        plugin
            .handle_command("set_transition", serde_json::json!({"name": "Fade"}))
            .unwrap();
        plugin
            .handle_command(
                "set_scene_item_enabled",
                serde_json::json!({"scene_name": "Escena", "item_id": 1, "enabled": false}),
            )
            .unwrap();

        let transitions = plugin
            .handle_command("get_transitions", serde_json::json!({}))
            .unwrap();
        let items = plugin
            .handle_command(
                "get_scene_items",
                serde_json::json!({"scene_name": "Escena"}),
            )
            .unwrap();
        assert_eq!(transitions[1]["kind"], "custom");
        assert!(!items[0]["enabled"].as_bool().unwrap());
    }

    #[test]
    fn disconnect_command_resets_interface_data() {
        let mut plugin = ObsPlugin::with_test_controller(test_state());
        plugin
            .handle_command(
                "connect",
                serde_json::json!({"host": "127.0.0.1", "port": 4455, "password": null}),
            )
            .unwrap();

        plugin
            .handle_command("disconnect", serde_json::json!({}))
            .unwrap();

        assert!(!plugin.data.connected);
        assert_eq!(plugin.data.current_scene, "");
    }
}
