use plugin_system::{Plugin, PluginMetadata};
use std::collections::HashMap;

pub struct TimerPlugin {
    timers: HashMap<String, u64>,
}

impl Default for TimerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerPlugin {
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
        }
    }

    pub fn start_timer(&mut self, name: String, seconds: u64) {
        self.timers.insert(name, seconds);
    }

    pub fn get_timer(&self, name: &str) -> Option<u64> {
        self.timers.get(name).copied()
    }

    pub fn list_timers(&self) -> Vec<String> {
        self.timers.keys().cloned().collect()
    }

    pub fn interface_ids(&self) -> Vec<&'static str> {
        vec!["Timer"]
    }
}

#[plugin_system::plugin_export("timer")]
impl Plugin for TimerPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "timer",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("TimerPlugin loaded");
    }

    fn on_unload(&mut self) {
        log::info!("TimerPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
