use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::traits::Plugin;

/// Trait for command handlers that can be registered by plugins.
///
/// Plugins implement this trait to expose their commands to the API layer
/// without requiring type-based downcasting.
pub trait CommandHandler: Send + Sync {
    /// Handle a command by name with JSON arguments.
    fn handle_command(
        &mut self,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value>;

    /// List all commands this handler supports.
    fn commands(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

/// A command handler that wraps a `dyn Plugin` and delegates to its `handle_command`.
pub struct PluginCommandHandler {
    plugin: Arc<RwLock<Box<dyn Plugin>>>,
}

impl PluginCommandHandler {
    pub fn new(plugin: Arc<RwLock<Box<dyn Plugin>>>) -> Self {
        Self { plugin }
    }
}

impl CommandHandler for PluginCommandHandler {
    fn handle_command(
        &mut self,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        let mut plugin = self.plugin.write().expect("plugin lock poisoned");
        plugin.handle_command(method, args)
    }
}

/// Registry for command handlers, keyed by plugin name.
pub struct CommandRegistry {
    handlers: HashMap<String, Arc<RwLock<Box<dyn CommandHandler>>>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a command handler for a plugin.
    pub fn register(&mut self, plugin_name: &str, handler: Box<dyn CommandHandler>) {
        self.handlers
            .insert(plugin_name.to_string(), Arc::new(RwLock::new(handler)));
    }

    /// Get a command handler by plugin name.
    pub fn get_handler(&self, plugin_name: &str) -> Option<Arc<RwLock<Box<dyn CommandHandler>>>> {
        self.handlers.get(plugin_name).cloned()
    }

    /// Check if a handler is registered for a plugin.
    pub fn has_handler(&self, plugin_name: &str) -> bool {
        self.handlers.contains_key(plugin_name)
    }

    /// List all registered plugin names.
    pub fn plugin_names(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a handler for a plugin.
    pub fn unregister(&mut self, plugin_name: &str) -> bool {
        self.handlers.remove(plugin_name).is_some()
    }

    /// Handle a command by delegating to the appropriate handler.
    pub fn handle_command(
        &self,
        plugin_name: &str,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        let handler = self.get_handler(plugin_name)?;
        let mut handler = handler.write().expect("handler lock poisoned");
        handler.handle_command(method, args)
    }

    /// Convenience method: call a command and deserialize the result.
    pub fn call_command<T: serde::de::DeserializeOwned>(
        &self,
        plugin_name: &str,
        method: &str,
        args: serde_json::Value,
    ) -> Option<T> {
        let result = self.handle_command(plugin_name, method, args)?;
        serde_json::from_value(result).ok()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared reference to a command registry.
pub type SharedCommandRegistry = Arc<RwLock<CommandRegistry>>;

/// Create a new shared command registry.
pub fn new_shared_command_registry() -> SharedCommandRegistry {
    Arc::new(RwLock::new(CommandRegistry::new()))
}
