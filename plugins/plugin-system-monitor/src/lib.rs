use plugin_system::{Plugin, PluginMetadata};

pub struct SystemMonitorPlugin {
    cpu_usage: f64,
    memory_usage: f64,
}

impl Default for SystemMonitorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitorPlugin {
    pub fn new() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
        }
    }

    pub fn get_cpu_usage(&self) -> f64 {
        // Simulated - in real implementation would use sysinfo crate
        self.cpu_usage
    }

    pub fn get_memory_usage(&self) -> f64 {
        // Simulated - in real implementation would use sysinfo crate
        self.memory_usage
    }

    pub fn update_metrics(&mut self) {
        // Simulated metrics update
        self.cpu_usage = 45.2;
        self.memory_usage = 62.8;
    }
}

#[plugin_system::plugin_export]
impl Plugin for SystemMonitorPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "system-monitor",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("SystemMonitorPlugin loaded");
        self.update_metrics();
    }

    fn on_unload(&mut self) {
        log::info!("SystemMonitorPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["ResourceInfo"]
    }
}
