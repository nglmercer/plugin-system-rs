use plugin_system::{PluginContext, PluginMetadata};
use std::collections::HashMap;

pub struct TimerPlugin {
    timers: HashMap<String, u64>,
}

impl Default for TimerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[plugin_system::plugin_export]
impl TimerPlugin {
    pub fn new() -> Self {
        Self {
            timers: HashMap::new(),
        }
    }

    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "timer",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        log::info!("TimerPlugin loaded");
    }

    fn on_unload(&mut self) {
        log::info!("TimerPlugin unloading");
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_system::Plugin;

    #[test]
    fn metadata_and_interface_ids_are_generated() {
        let plugin = TimerPlugin::new();

        assert_eq!(plugin.metadata().name, "timer");
        assert_eq!(plugin.interface_ids(), vec!["Timer"]);
        assert!(plugin.interface_data().is_none());
    }

    #[test]
    fn timer_helpers_manage_timers() {
        let mut plugin = TimerPlugin::new();

        plugin.start_timer("standup".to_string(), 900);

        assert_eq!(plugin.get_timer("standup"), Some(900));
        assert_eq!(plugin.list_timers(), vec!["standup".to_string()]);
    }
}
