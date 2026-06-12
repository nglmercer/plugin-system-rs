use plugin_system::{Plugin, PluginContext, PluginMetadata};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
use unsupported as platform;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeState {
    pub master_volume: f32,
    pub muted: bool,
    pub default_device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVolume {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeData {
    pub state: VolumeState,
    #[serde(default)]
    pub apps: Vec<AppVolume>,
    pub platform_supported: bool,
    pub per_app_supported: bool,
}

pub trait VolumeControl: Send + Sync {
    fn get_master_volume(&mut self) -> Result<VolumeState, String>;
    fn set_master_volume(&mut self, volume: f32) -> Result<(), String>;
    fn set_muted(&mut self, muted: bool) -> Result<(), String>;
    fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String>;
    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String>;
    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String>;
}

pub struct VolumeMasterPlugin {
    controller: Box<dyn VolumeControl>,
    pub data: VolumeData,
}

impl Default for VolumeMasterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VolumeMasterPlugin {
    pub fn new() -> Self {
        let mut controller = platform::create_controller();
        let data = controller
            .get_master_volume()
            .map(|state| VolumeData {
                state,
                apps: controller.get_app_volumes().unwrap_or_default(),
                platform_supported: true,
                per_app_supported: platform::per_app_supported(),
            })
            .unwrap_or(VolumeData {
                platform_supported: false,
                per_app_supported: false,
                ..Default::default()
            });

        Self { controller, data }
    }

    pub fn interface_ids(&self) -> Vec<&'static str> {
        vec!["VolumeMaster"]
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
            "refresh" => {
                self.refresh();
                Some(serde_json::json!({"ok": true}))
            }
            "set_volume" => {
                let volume = args.get("volume").and_then(|v| v.as_f64()).unwrap_or(50.0) as f32;
                match self.set_volume(volume) {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "set_mute" => {
                let muted = args.get("muted").and_then(|v| v.as_bool()).unwrap_or(false);
                match self.set_muted(muted) {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "set_app_volume" => {
                let app_name = args.get("app_name").and_then(|v| v.as_str()).unwrap_or("");
                let volume = args.get("volume").and_then(|v| v.as_f64()).unwrap_or(50.0) as f32;
                match self.set_app_volume(app_name, volume) {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            "set_app_mute" => {
                let app_name = args.get("app_name").and_then(|v| v.as_str()).unwrap_or("");
                let muted = args.get("muted").and_then(|v| v.as_bool()).unwrap_or(false);
                match self.set_app_muted(app_name, muted) {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
                }
            }
            _ => None,
        }
    }

    pub fn refresh(&mut self) {
        if let Ok(state) = self.controller.get_master_volume() {
            self.data.state = state;
        }
        if let Ok(apps) = self.controller.get_app_volumes() {
            self.data.apps = apps;
        }
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        let clamped = volume.clamp(0.0, 100.0);
        self.controller.set_master_volume(clamped)?;
        self.data.state.master_volume = clamped;
        Ok(())
    }

    pub fn set_muted(&mut self, muted: bool) -> Result<(), String> {
        self.controller.set_muted(muted)?;
        self.data.state.muted = muted;
        Ok(())
    }

    pub fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
        let clamped = volume.clamp(0.0, 100.0);
        self.controller.set_app_volume(app_name, clamped)?;
        if let Some(app) = self.data.apps.iter_mut().find(|a| a.name == app_name) {
            app.volume = clamped;
        }
        Ok(())
    }

    pub fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
        self.controller.set_app_muted(app_name, muted)?;
        if let Some(app) = self.data.apps.iter_mut().find(|a| a.name == app_name) {
            app.muted = muted;
        }
        Ok(())
    }
}

#[plugin_system::plugin_export]
impl Plugin for VolumeMasterPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "volume-master",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        log::info!("VolumeMasterPlugin loaded");
        self.refresh();
    }

    fn on_unload(&mut self) {
        log::info!("VolumeMasterPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
