use crate::handler::{CommandHandler, SharedCommandRegistry};
use crate::registry::{PluginRegistry, SharedRegistry};
use crate::traits::Plugin;
use std::sync::Arc;

pub struct PluginContext {
    pub registry: SharedRegistry,
    pub command_registry: SharedCommandRegistry,
}

impl PluginContext {
    pub fn new(registry: SharedRegistry, command_registry: SharedCommandRegistry) -> Self {
        Self {
            registry,
            command_registry,
        }
    }

    pub fn registry(&self) -> std::sync::RwLockReadGuard<'_, PluginRegistry> {
        self.registry
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn registry_mut(&self) -> std::sync::RwLockWriteGuard<'_, PluginRegistry> {
        self.registry
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn command_registry(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, crate::handler::CommandRegistry> {
        self.command_registry
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn command_registry_mut(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, crate::handler::CommandRegistry> {
        self.command_registry
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Register a command handler for a plugin.
    pub fn register_handler(&self, plugin_name: &str, handler: Box<dyn CommandHandler>) {
        let mut registry = self.command_registry_mut();
        registry.register(plugin_name, handler);
    }

    pub fn get_plugin(&self, name: &str) -> Option<Arc<std::sync::RwLock<Box<dyn Plugin>>>> {
        let registry = self
            .registry
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        registry.get_by_name(name)
    }

    pub fn with_plugin<R>(&self, name: &str, f: impl FnOnce(&dyn Plugin) -> R) -> Option<R> {
        let registry = self
            .registry
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let plugin_arc = registry.get_by_name(name)?;
        let guard = plugin_arc
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let plugin_ref: &dyn Plugin = &**guard;
        Some(f(plugin_ref))
    }

    pub fn with_plugin_mut<R>(
        &self,
        name: &str,
        f: impl FnOnce(&mut dyn Plugin) -> R,
    ) -> Option<R> {
        let registry = self
            .registry
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let plugin_arc = registry.get_by_name(name)?;
        let mut guard = plugin_arc
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let plugin_ref: &mut dyn Plugin = &mut **guard;
        Some(f(plugin_ref))
    }
}
