use plugin_system::{Plugin, PluginMetadata};
use std::collections::HashMap;

pub struct HotkeyPlugin {
    bindings: HashMap<String, String>,
}

impl Default for HotkeyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyPlugin {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn add_binding(&mut self, name: String, keys: String) {
        self.bindings.insert(name, keys);
    }

    pub fn get_binding(&self, name: &str) -> Option<&String> {
        self.bindings.get(name)
    }

    pub fn list_bindings(&self) -> Vec<String> {
        self.bindings.keys().cloned().collect()
    }
}

#[plugin_system::plugin_export]
impl Plugin for HotkeyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "hotkey",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("HotkeyPlugin loaded");
        // Add some default bindings
        self.add_binding("copy".to_string(), "ctrl+c".to_string());
        self.add_binding("paste".to_string(), "ctrl+v".to_string());
        self.add_binding("cut".to_string(), "ctrl+x".to_string());
    }

    fn on_unload(&mut self) {
        log::info!("HotkeyPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["HotkeyManager"]
    }
}
