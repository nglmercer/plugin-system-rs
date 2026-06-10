use plugin_interfaces::{DataProvider, Greet};
use plugin_system::{Plugin, PluginMetadata};

pub struct HelloPluginWrapper(plugin_types::HelloPlugin);

impl Default for HelloPluginWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl HelloPluginWrapper {
    pub fn new() -> Self {
        Self(plugin_types::HelloPlugin::new())
    }

    pub fn set_greeting(&mut self, greeting: String) {
        self.0.set_greeting(greeting);
    }

    pub fn get_greeting(&self) -> &str {
        self.0.get_greeting()
    }

    pub fn add_data(&mut self, key: String, value: String) {
        self.0.add_data(key, value);
    }

    pub fn get_data_value(&self, key: &str) -> Option<&String> {
        self.0.get_data_value(key)
    }
}

#[plugin_system::plugin_export]
impl Plugin for HelloPluginWrapper {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "hello",
            version: "0.1.0",
            authors: ["Plugin System Examples"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("HelloPlugin loaded");
    }

    fn on_unload(&mut self) {
        log::info!("HelloPlugin unloading...");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["Greet", "DataProvider"]
    }

    fn handle_command(&mut self, command: &str) -> String {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        let method = parts[0];
        let args = parts.get(1).unwrap_or(&"");

        match method {
            "help" => "Available commands:\n\
                 \t  help              - Show this help message\n\
                 \t  greet <name>      - Greet someone\n\
                 \t  set_greeting <g>  - Set the greeting word\n\
                 \t  get_greeting      - Get current greeting word\n\
                 \t  info              - Get plugin info"
                .to_string(),
            "greet" => {
                let name = if args.is_empty() { "World" } else { args };
                self.0.greet(name)
            }
            "set_greeting" => {
                if args.is_empty() {
                    "ERROR: Usage: set_greeting <greeting>".to_string()
                } else {
                    self.0.set_greeting(args.to_string());
                    format!("OK: Greeting set to '{}'", args)
                }
            }
            "get_greeting" => format!("Current greeting: '{}'", self.0.get_greeting()),
            "info" => format!(
                "Plugin: {} v{}\nGreeting: '{}'",
                self.metadata().name,
                self.metadata().version,
                self.0.get_greeting()
            ),
            _ => format!(
                "ERROR: Unknown method '{}'. Type 'help' for available commands.",
                method
            ),
        }
    }
}

impl Greet for HelloPluginWrapper {
    fn greet(&self, name: &str) -> String {
        self.0.greet(name)
    }

    fn get_name(&self) -> &str {
        "HelloPlugin"
    }
}

impl DataProvider for HelloPluginWrapper {
    fn get_data(&self, key: &str) -> Option<String> {
        self.0.get_data(key)
    }

    fn set_data(&mut self, key: &str, value: String) {
        self.0.set_data(key, value);
    }

    fn list_keys(&self) -> Vec<String> {
        self.0.list_keys()
    }
}
