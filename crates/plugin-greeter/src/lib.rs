use plugin_interfaces::{Calculator, GreetingService, Logger};
use plugin_system::{Plugin, PluginMetadata};

pub struct GreeterPluginWrapper(plugin_types::GreeterPlugin);

impl Default for GreeterPluginWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl GreeterPluginWrapper {
    pub fn new() -> Self {
        Self(plugin_types::GreeterPlugin::new())
    }

    pub fn get_count(&self) -> u64 {
        self.0.get_count()
    }

    pub fn get_last_greeting(&self) -> &str {
        self.0.get_last_greeting()
    }
}

#[plugin_system::plugin_export]
impl Plugin for GreeterPluginWrapper {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "greeter",
            version: "0.1.0",
            authors: ["Plugin System Examples"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("GreeterPlugin loaded");
    }

    fn on_unload(&mut self) {
        log::info!("GreeterPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["GreetingService", "Calculator", "Logger"]
    }

    fn handle_command(&mut self, command: &str) -> String {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        let method = parts[0];
        let args = parts.get(1).unwrap_or(&"");

        match method {
            "help" => "Available commands:\n\
                 \t  help              - Show this help message\n\
                 \t  greet <name>      - Greet someone (counts)\n\
                 \t  count             - Get total greeting count\n\
                 \t  last              - Get last greeting\n\
                 \t  info              - Get plugin info"
                .to_string(),
            "greet" => {
                let name = if args.is_empty() { "World" } else { args };
                self.0.greet(name)
            }
            "count" => format!("Total greetings: {}", self.0.count_greetings()),
            "last" => {
                let last = self.0.get_last_greeting();
                if last.is_empty() {
                    "No greetings yet".to_string()
                } else {
                    format!("Last greeting: {}", last)
                }
            }
            "info" => format!(
                "Plugin: {} v{}\nTotal greetings: {}",
                self.metadata().name,
                self.metadata().version,
                self.0.get_count()
            ),
            _ => format!(
                "ERROR: Unknown method '{}'. Type 'help' for available commands.",
                method
            ),
        }
    }
}

impl GreetingService for GreeterPluginWrapper {
    fn greet(&self, name: &str) -> String {
        self.0.greet(name)
    }

    fn count_greetings(&self) -> u64 {
        self.0.count_greetings()
    }
}

impl Calculator for GreeterPluginWrapper {
    fn add(&self, a: f64, b: f64) -> f64 {
        self.0.add(a, b)
    }

    fn subtract(&self, a: f64, b: f64) -> f64 {
        self.0.subtract(a, b)
    }

    fn multiply(&self, a: f64, b: f64) -> f64 {
        self.0.multiply(a, b)
    }

    fn divide(&self, a: f64, b: f64) -> Option<f64> {
        self.0.divide(a, b)
    }
}

impl Logger for GreeterPluginWrapper {
    fn log_info(&self, message: &str) {
        self.0.log_info(message);
    }

    fn log_error(&self, message: &str) {
        self.0.log_error(message);
    }

    fn get_logs(&self) -> Vec<String> {
        self.0.get_logs()
    }
}
