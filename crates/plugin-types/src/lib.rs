use plugin_interfaces::{Greet, GreetingService};
use plugin_system::{Plugin, PluginMetadata};

/// The Hello plugin - a simple example that other plugins can depend on.
pub struct HelloPlugin {
    greeting: String,
}

impl Default for HelloPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HelloPlugin {
    pub fn new() -> Self {
        Self {
            greeting: "Hello".to_string(),
        }
    }
}

impl Plugin for HelloPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "hello",
            version: "0.1.0",
            authors: ["Plugin System Examples"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("HelloPlugin loaded with greeting: '{}'", self.greeting);
    }

    fn on_unload(&mut self) {
        log::info!("HelloPlugin unloading...");
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
                self.greet(name)
            }
            "set_greeting" => {
                if args.is_empty() {
                    "ERROR: Usage: set_greeting <greeting>".to_string()
                } else {
                    self.greeting = args.to_string();
                    log::info!("Greeting updated to: '{}'", self.greeting);
                    format!("OK: Greeting set to '{}'", self.greeting)
                }
            }
            "get_greeting" => format!("Current greeting: '{}'", self.greeting),
            "info" => format!(
                "Plugin: {} v{}\nGreeting: '{}'",
                self.metadata().name,
                self.metadata().version,
                self.greeting
            ),
            _ => format!(
                "ERROR: Unknown method '{}'. Type 'help' for available commands.",
                method
            ),
        }
    }
}

impl Greet for HelloPlugin {
    fn greet(&self, name: &str) -> String {
        format!("{}, {}!", self.greeting, name)
    }

    fn get_name(&self) -> &str {
        "HelloPlugin"
    }
}

/// The Greeter plugin - demonstrates plugin lifecycle and capabilities.
pub struct GreeterPlugin {
    greeting_count: u64,
    last_greeting: String,
}

impl Default for GreeterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GreeterPlugin {
    pub fn new() -> Self {
        Self {
            greeting_count: 0,
            last_greeting: String::new(),
        }
    }
}

impl Plugin for GreeterPlugin {
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
        log::info!(
            "GreeterPlugin unloading after {} greetings",
            self.greeting_count
        );
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
                self.greeting_count += 1;
                self.last_greeting = self.greet(name);
                log::info!("Greeting #{}: {}", self.greeting_count, self.last_greeting);
                self.last_greeting.clone()
            }
            "count" => format!("Total greetings: {}", self.greeting_count),
            "last" => {
                if self.last_greeting.is_empty() {
                    "No greetings yet".to_string()
                } else {
                    format!("Last greeting: {}", self.last_greeting)
                }
            }
            "info" => format!(
                "Plugin: {} v{}\nTotal greetings: {}",
                self.metadata().name,
                self.metadata().version,
                self.greeting_count
            ),
            _ => format!(
                "ERROR: Unknown method '{}'. Type 'help' for available commands.",
                method
            ),
        }
    }
}

impl GreetingService for GreeterPlugin {
    fn greet(&self, name: &str) -> String {
        format!("Welcome, {}! (via GreeterPlugin)", name)
    }

    fn count_greetings(&self) -> u64 {
        self.greeting_count
    }
}
