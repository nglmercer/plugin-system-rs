use plugin_interfaces::{Greet, GreetingService};
use plugin_system::{Plugin, PluginMetadata};

/// The Hello plugin - a simple example that other plugins can depend on.
pub struct HelloPlugin {
    greeting: String,
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
}

impl GreeterPlugin {
    pub fn new() -> Self {
        Self {
            greeting_count: 0,
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
}

impl GreetingService for GreeterPlugin {
    fn greet(&self, name: &str) -> String {
        format!("Welcome, {}! (via GreeterPlugin)", name)
    }

    fn count_greetings(&self) -> u64 {
        self.greeting_count
    }
}
